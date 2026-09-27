//! Claude 提示词注入（Tauri command 薄壳）。

use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::prompt_backups::create_claude_prompt_backup;
use crate::devconduit::prompts::{
    claude_builtin_prompt_content, get_saved_prompt_inner, install_managed_claude_block,
    uninstall_managed_claude_block, PromptInjectionMode, ENGINE_CLAUDE,
};
use crate::devconduit::state::{build_claude_state, ClaudeActionResult};

fn enable_claude_prompt_content_inner(
    filename: &str,
    content: &str,
    template_key: &str,
    title: &str,
    content_source: &str,
    injection_mode: PromptInjectionMode,
    action: &str,
) -> Result<ClaudeActionResult> {
    if filename.trim().is_empty()
        || !filename.to_ascii_lowercase().ends_with(".md")
        || filename.contains('/')
        || filename.contains('\\')
    {
        return Err(CodexxError::Config("提示词文件名无效".to_string()));
    }
    if template_key.trim().is_empty() || template_key.contains("-->") {
        return Err(CodexxError::Config("提示词模板标识无效".to_string()));
    }

    let backup_id = create_claude_prompt_backup(action)?;
    install_managed_claude_block(template_key, filename, content, injection_mode)?;

    let state = build_claude_state()?;
    Ok(ClaudeActionResult {
        ok: true,
        message: format!(
            "已用{}模式启用 {title}（来源：{content_source}）",
            if injection_mode == PromptInjectionMode::Append {
                "保留"
            } else {
                "替换"
            },
        ),
        backup_id,
        state,
    })
}

fn enable_claude_instruction_inner(
    template_id: &str,
    injection_mode: Option<String>,
) -> Result<ClaudeActionResult> {
    let resolved_id = if template_id.trim().is_empty() {
        "claude-project-rules"
    } else {
        template_id.trim()
    };
    let (filename, _relative, content, content_source) = claude_builtin_prompt_content(resolved_id)?;
    let mode = PromptInjectionMode::parse(injection_mode.as_deref())?;
    enable_claude_prompt_content_inner(
        &filename,
        &content,
        &format!("builtin:{resolved_id}"),
        &filename,
        &content_source,
        mode,
        "enable-claude-instruct",
    )
}

fn enable_claude_saved_prompt_inner(
    id: String,
    injection_mode: Option<String>,
) -> Result<ClaudeActionResult> {
    let prompt = get_saved_prompt_inner(id.trim(), ENGINE_CLAUDE)?;
    let mode = PromptInjectionMode::parse(injection_mode.as_deref())?;
    enable_claude_prompt_content_inner(
        &prompt.filename,
        &prompt.content,
        &format!("saved:{}", prompt.id),
        &prompt.title,
        "本地自定义",
        mode,
        "enable-claude-custom-prompt",
    )
}

fn disable_claude_instruction_inner() -> Result<ClaudeActionResult> {
    let backup_id = create_claude_prompt_backup("disable-claude-instruct")?;
    let removed = uninstall_managed_claude_block()?;
    let state = build_claude_state()?;
    Ok(ClaudeActionResult {
        ok: true,
        message: if removed {
            "已禁用 Claude 指令提示词".to_string()
        } else {
            "当前没有启用的受管 Claude 指令".to_string()
        },
        backup_id,
        state,
    })
}

#[tauri::command]
pub(crate) async fn dc_claude_prompt_state() -> Result<crate::devconduit::state::ClaudeState> {
    tauri::async_runtime::spawn_blocking(build_claude_state)
        .await
        .map_err(|e| CodexxError::Config(format!("读取 Claude 提示词状态失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_enable_claude_builtin(
    template_id: Option<String>,
    injection_mode: Option<String>,
) -> Result<ClaudeActionResult> {
    tauri::async_runtime::spawn_blocking(move || {
        enable_claude_instruction_inner(
            template_id.as_deref().unwrap_or("claude-project-rules"),
            injection_mode,
        )
    })
    .await
    .map_err(|e| CodexxError::Config(format!("启用 Claude 模板提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_enable_claude_saved(
    id: String,
    injection_mode: Option<String>,
) -> Result<ClaudeActionResult> {
    tauri::async_runtime::spawn_blocking(move || enable_claude_saved_prompt_inner(id, injection_mode))
        .await
        .map_err(|e| CodexxError::Config(format!("启用 Claude 自定义提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_disable_claude_instruction() -> Result<ClaudeActionResult> {
    tauri::async_runtime::spawn_blocking(disable_claude_instruction_inner)
        .await
        .map_err(|e| CodexxError::Config(format!("禁用 Claude 指令提示词失败: {e}")))?
}
