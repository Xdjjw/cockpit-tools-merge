//! DevConduit 三大能力（提示词注入 / MCP 自动挂载 / Skills 挂载）的 Cockpit 移植。
//!
//! 只支持 Codex 与 Claude 两个渠道；数据存 Cockpit 独立库 `~/.cockpit/devconduit.db`。

pub(crate) mod constants;
pub(crate) mod db;
pub(crate) mod error;
pub(crate) mod file_io;
pub(crate) mod import_devconduit;
pub(crate) mod multitool_stubs;
pub(crate) mod paths;
pub(crate) mod platform;
pub(crate) mod prompt_backups;
pub(crate) mod prompts;
pub(crate) mod remote;
pub(crate) mod skills_commands;
pub(crate) mod skills_mcp;
pub(crate) mod sqlite_utils;
pub(crate) mod state;
pub(crate) mod toml_utils;
pub(crate) mod tools;

pub(crate) use db::open_db;
pub(crate) use multitool_stubs::ccswitch;
pub(crate) use multitool_stubs::pi;
pub(crate) use multitool_stubs::providers;
pub(crate) use multitool_stubs::zcode;
pub(crate) use toml_utils::string_value;
pub(crate) use prompts::ENGINE_CLAUDE;
pub(crate) use prompts::ENGINE_CODEX;

use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::file_io::{io_err, write_text};
use crate::devconduit::paths::home_dir;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{value, Document};

pub(crate) fn now_rfc3339() -> String {
    chrono::Local::now().to_rfc3339()
}

pub(crate) fn sanitize_id(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.trim().to_ascii_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        format!("provider-{}", chrono::Local::now().timestamp_millis())
    } else {
        out
    }
}

pub(crate) fn config_path(codex_dir: &Path) -> PathBuf {
    codex_dir.join("config.toml")
}

pub(crate) fn auth_path(codex_dir: &Path) -> PathBuf {
    codex_dir.join("auth.json")
}

fn default_codex_dir() -> Result<PathBuf> {
    if let Ok(value) = std::env::var("CODEX_HOME") {
        if let Some(path) = codex_dir_from_text(&value)? {
            return Ok(path);
        }
    }
    Ok(home_dir()?.join(".codex"))
}

fn codex_dir_from_text(value: &str) -> Result<Option<PathBuf>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let unquoted = if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    if unquoted.trim().is_empty() {
        return Ok(None);
    }
    if unquoted == "~" {
        return Ok(Some(home_dir()?));
    }
    if let Some(rest) = unquoted.strip_prefix("~/").or_else(|| unquoted.strip_prefix("~\\")) {
        return Ok(Some(home_dir()?.join(rest)));
    }
    Ok(Some(PathBuf::from(unquoted)))
}

#[cfg(target_os = "windows")]
fn resolve_windows_linked_directory(path: PathBuf) -> Result<PathBuf> {
    use std::os::windows::fs::FileTypeExt;

    let original = path.clone();
    let mut current = path;
    let mut followed_link = false;
    let mut visited = HashSet::new();
    for _ in 0..16 {
        if !visited.insert(current.clone()) {
            return Err(CodexxError::Config(format!(
                "当前 Codex 目录链接形成了循环：{}",
                original.display()
            )));
        }
        let metadata = match fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !followed_link => {
                return Ok(current);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(CodexxError::Config(format!(
                    "当前 Codex 目录链接的目标不存在：{}",
                    original.display()
                )));
            }
            Err(error) => return Err(io_err(&current, error)),
        };
        let file_type = metadata.file_type();
        if metadata.is_dir() && !file_type.is_symlink_dir() {
            return Ok(current);
        }
        if file_type.is_symlink_file() || file_type.is_symlink_dir() || file_type.is_symlink() {
            let target = fs::read_link(&current).map_err(|error| io_err(&current, error))?;
            current = if target.is_absolute() {
                target
            } else {
                current.parent().map(|parent| parent.join(&target)).unwrap_or(target)
            };
            followed_link = true;
            continue;
        }
        return Err(CodexxError::Config(format!(
            "当前 CODEX_HOME 不是文件夹：{}",
            original.display()
        )));
    }
    Err(CodexxError::Config(format!(
        "当前 Codex 目录链接层级过多：{}",
        original.display()
    )))
}

#[cfg(not(target_os = "windows"))]
fn resolve_windows_linked_directory(path: PathBuf) -> Result<PathBuf> {
    Ok(path)
}

pub(crate) fn resolve_codex_dir(config_dir: Option<String>) -> Result<PathBuf> {
    let path = match config_dir.as_deref().map(codex_dir_from_text).transpose()? {
        Some(Some(path)) => Ok(path),
        _ => default_codex_dir(),
    }?;
    resolve_windows_linked_directory(path)
}

// ---------------------------------------------------------------------------
// Codex 提示词注入（Tauri command 薄壳 + 核心逻辑）
// ---------------------------------------------------------------------------

use crate::devconduit::file_io::{ensure_directory, parse_toml_document, read_to_string_if_exists};
use crate::devconduit::prompt_backups::create_codex_prompt_backup;
use crate::devconduit::prompts::{
    agents_path, builtin_prompt_content, managed_agents_bounds, managed_model_instruction_path,
    prompt_template_key_for_instruction,
    resolve_instruction_path, uninstall_managed_agents_block, PromptInjectionMode,
};
use crate::devconduit::state::ActionResult;

#[allow(clippy::too_many_arguments)]
fn enable_prompt_content_inner(
    config_dir: Option<String>,
    filename: &str,
    content: &str,
    template_key: &str,
    title: &str,
    content_source: &str,
    injection_mode: PromptInjectionMode,
    action: &str,
) -> Result<ActionResult> {
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

    let codex_dir = resolve_codex_dir(config_dir)?;
    ensure_directory(&codex_dir)?;
    let cfg = config_path(&codex_dir);
    let agents = agents_path(&codex_dir);
    let text = read_to_string_if_exists(&cfg)?;
    let mut doc = parse_toml_document(&cfg, &text)?;
    let agents_text = read_to_string_if_exists(&agents)?;
    managed_agents_bounds(&agents_text)?;
    let previous_managed_file = managed_model_instruction_path(&codex_dir, &doc)?;
    // 注意：不再调用 remember_current_instruction_prompt —— 启用动作不得在自定义区
    // 产生任何条目（用户反馈：内置模板行为必须与默认模板完全一致）。切回外部
    // 提示词的兜底由 prompt_backups 快照承担。
    let backup_id = create_codex_prompt_backup(&codex_dir, action)?;
    if crate::devconduit::prompts::lskill::agents_file_is_lskill_159(&codex_dir)? {
        write_text(
            &agents,
            "# Codex\n\n<!-- lskill-1.5.9 replaced -->\n",
        )?;
    }
    match injection_mode {
        PromptInjectionMode::Replace => {
            if doc.get("model").is_none() {
                doc["model"] = value("gpt-5.5");
            }
            doc["model_instructions_file"] = value(format!("./{filename}"));
            write_text(&codex_dir.join(filename), content)?;
            write_text(&cfg, &doc.to_string())?;
            uninstall_managed_agents_block(&codex_dir)?;
        }
        PromptInjectionMode::Append => {
            crate::devconduit::prompts::install_managed_agents_block(
                &codex_dir,
                template_key,
                content,
            )?;
            if previous_managed_file.is_some() {
                doc.as_table_mut().remove("model_instructions_file");
                write_text(&cfg, &doc.to_string())?;
            }
        }
    }

    if let Some(previous) = previous_managed_file {
        let next = codex_dir.join(filename);
        let should_remove = injection_mode == PromptInjectionMode::Append || previous != next;
        if should_remove && previous.parent() == Some(codex_dir.as_path()) && previous.exists() {
            fs::remove_file(&previous).map_err(|e| io_err(&previous, e))?;
        }
    }

    let state = crate::devconduit::state::build_state(codex_dir)?;
    Ok(ActionResult {
        ok: true,
        message: format!(
            "已用{}模式启用 {title}（来源：{content_source}）",
            if injection_mode == PromptInjectionMode::Append {
                "追加"
            } else {
                "替换"
            }
        ),
        backup_id,
        state,
    })
}

/// 受管 rules 文件统一前缀（区别于用户自己的 .rules，避免误删）。
const MANAGED_RULES_PREFIX: &str = "dc-";

fn managed_rules_path(codex_dir: &Path, prompt_id: &str) -> PathBuf {
    codex_dir.join("rules").join(format!(
        "{}{}.rules",
        MANAGED_RULES_PREFIX,
        crate::devconduit::sanitize_id(prompt_id)
    ))
}

/// 清空全部受管 rules（前缀匹配），返回删除数量。
pub(crate) fn remove_managed_rules(codex_dir: &Path) -> usize {
    let dir = codex_dir.join("rules");
    let Ok(entries) = fs::read_dir(&dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with(MANAGED_RULES_PREFIX) && name.ends_with(".rules") {
            if fs::remove_file(entry.path()).is_ok() {
                removed += 1;
            }
        }
    }
    removed
}

/// 写入本条提示词的配套 rules（如有），并清掉其他受管 rules。
fn sync_managed_rules(codex_dir: &Path, prompt_id: &str, rules: Option<&str>) -> Result<()> {
    remove_managed_rules(codex_dir);
    if let Some(rules) = rules.filter(|r| !r.trim().is_empty()) {
        let path = managed_rules_path(codex_dir, prompt_id);
        if let Some(parent) = path.parent() {
            ensure_directory(parent)?;
        }
        write_text(&path, rules)?;
    }
    Ok(())
}

fn enable_saved_prompt_inner(
    config_dir: Option<String>,
    id: String,
    injection_mode: Option<String>,
) -> Result<ActionResult> {
    let prompt = crate::devconduit::prompts::store::get_saved_prompt_inner(id.trim(), ENGINE_CODEX)?;
    let mode = PromptInjectionMode::parse(injection_mode.as_deref())?;
    let result = enable_prompt_content_inner(
        config_dir.clone(),
        &prompt.filename,
        &prompt.content,
        &format!("saved:{}", prompt.id),
        &prompt.title,
        "本地自定义",
        mode,
        "enable-custom-prompt",
    )?;
    // 配套 rules：与提示词同动作写入（见 07-workshop/PROMPT-INJECTION.md §5）
    let codex_dir = resolve_codex_dir(config_dir)?;
    if let Err(error) = sync_managed_rules(&codex_dir, &prompt.id, prompt.rules_content.as_deref()) {
        return Err(CodexxError::Config(format!("写入配套 rules 失败: {error}")));
    }
    Ok(result)
}

fn enable_instruction_inner(
    config_dir: Option<String>,
    template_id: &str,
    injection_mode: Option<String>,
) -> Result<ActionResult> {
    let resolved_id = if template_id.trim().is_empty() {
        constants::CODEX_KEYSMITH_BUILTIN_ID
    } else {
        template_id.trim()
    };
    if resolved_id == constants::LSKILL_159_ID {
        let outcome = crate::devconduit::prompts::lskill::apply_lskill_159(config_dir)?;
        let state = crate::devconduit::state::build_state(outcome.codex_dir)?;
        return Ok(ActionResult {
            ok: true,
            message: outcome.message,
            backup_id: outcome.backup_id,
            state,
        });
    }
    let (filename, _relative, content, content_source) = builtin_prompt_content(resolved_id)?;
    let mode = PromptInjectionMode::parse(injection_mode.as_deref())?;
    let rules = crate::devconduit::prompts::catalog::bundled_prompt_meta(resolved_id)
        .and_then(|meta| meta.rules_content);
    let result = enable_prompt_content_inner(
        config_dir.clone(),
        &filename,
        &content,
        &format!("builtin:{resolved_id}"),
        &filename,
        &content_source,
        mode,
        "enable-instruct",
    )?;
    // 内置模板的配套 rules（如 ULTIMATE v3）与提示词成对写入（§5）
    if let Some(rules) = rules {
        let codex_dir = resolve_codex_dir(config_dir)?;
        sync_managed_rules(&codex_dir, resolved_id, Some(rules))?;
    }
    Ok(result)
}

fn disable_instruction_inner(
    config_dir: Option<String>,
    delete_file: Option<bool>,
) -> Result<ActionResult> {
    if crate::devconduit::prompts::lskill::agents_file_is_lskill_159(&resolve_codex_dir(config_dir.clone())?)? {
        let outcome = crate::devconduit::prompts::lskill::clear_lskill_159(config_dir)?;
        let state = crate::devconduit::state::build_state(outcome.codex_dir)?;
        return Ok(ActionResult {
            ok: true,
            message: outcome.message,
            backup_id: outcome.backup_id,
            state,
        });
    }
    let codex_dir = resolve_codex_dir(config_dir)?;
    let cfg = config_path(&codex_dir);
    let agents_text = read_to_string_if_exists(&agents_path(&codex_dir))?;
    managed_agents_bounds(&agents_text)?;
    let backup_id = create_codex_prompt_backup(&codex_dir, "disable-instruct")?;

    let text = read_to_string_if_exists(&cfg)?;
    let mut doc = parse_toml_document(&cfg, &text)?;
    let current = toml_utils::string_value(&doc, "model_instructions_file");
    let managed_model_path = managed_model_instruction_path(&codex_dir, &doc)?;
    let removed_model = managed_model_path.is_some();
    if removed_model {
        doc.as_table_mut().remove("model_instructions_file");
        write_text(&cfg, &doc.to_string())?;
    }
    let removed_agents = uninstall_managed_agents_block(&codex_dir)?;
    remove_managed_rules(&codex_dir);
    if delete_file.unwrap_or(true) {
        if let Some(md) = managed_model_path {
            if md.parent() == Some(codex_dir.as_path()) && md.exists() {
                fs::remove_file(&md).map_err(|e| io_err(&md, e))?;
            }
        }
    }

    let state = crate::devconduit::state::build_state(codex_dir)?;
    let removed = removed_model || removed_agents;
    Ok(ActionResult {
        ok: true,
        message: if removed {
            "已禁用指令提示词".to_string()
        } else if current.is_some() {
            "当前使用的是用户自己的提示词，未做修改".to_string()
        } else {
            "当前没有启用的受管提示词".to_string()
        },
        backup_id,
        state,
    })
}

fn disable_external_instruction_inner(config_dir: Option<String>) -> Result<ActionResult> {
    let codex_dir = resolve_codex_dir(config_dir)?;
    let cfg = config_path(&codex_dir);
    let text = read_to_string_if_exists(&cfg)?;
    let mut doc = parse_toml_document(&cfg, &text)?;
    let current = toml_utils::string_value(&doc, "model_instructions_file");
    if let Some(value) = current.as_deref() {
        if prompt_template_key_for_instruction(value)?.is_some() {
            return Err(CodexxError::Config(
                "当前是受管的提示词，请使用普通禁用按钮".to_string(),
            ));
        }
    }
    let backup_id = create_codex_prompt_backup(&codex_dir, "disable-external-instruct")?;
    if current.is_some() {
        doc.as_table_mut().remove("model_instructions_file");
        write_text(&cfg, &doc.to_string())?;
    }
    let state = crate::devconduit::state::build_state(codex_dir)?;
    Ok(ActionResult {
        ok: true,
        message: if current.is_some() {
            "已禁用外部提示词，原 md 文件已保留".to_string()
        } else {
            "当前没有外部提示词".to_string()
        },
        backup_id,
        state,
    })
}

// ---------------------------------------------------------------------------
// Tauri commands —— Codex 提示词
// ---------------------------------------------------------------------------

#[tauri::command]
pub(crate) async fn dc_codex_prompt_state(
    config_dir: Option<String>,
) -> Result<crate::devconduit::state::CodexState> {
    tauri::async_runtime::spawn_blocking(move || {
        let codex_dir = resolve_codex_dir(config_dir)?;
        crate::devconduit::state::build_state(codex_dir)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("读取 Codex 提示词状态失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_save_prompt(prompt: crate::devconduit::prompts::store::SavedPrompt) -> Result<crate::devconduit::prompts::store::SavedPrompt> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::devconduit::prompts::store::save_prompt_inner(prompt, ENGINE_CODEX)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("保存提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_delete_saved_prompt(id: String) -> Result<()> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::devconduit::prompts::store::delete_prompt_inner(id.trim(), ENGINE_CODEX)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("删除提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_list_saved_prompts(engine: Option<String>) -> Result<Vec<crate::devconduit::prompts::store::SavedPrompt>> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = engine.as_deref().unwrap_or(ENGINE_CODEX);
        crate::devconduit::prompts::store::list_saved_prompts_inner(engine)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("读取提示词列表失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_list_builtin_prompts() -> Result<Vec<crate::devconduit::prompts::BuiltinPromptStatus>> {
    tauri::async_runtime::spawn_blocking(crate::devconduit::prompts::catalog::builtin_prompt_status_inner)
        .await
        .map_err(|e| CodexxError::Config(format!("读取内置模板失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_enable_builtin_prompt(
    config_dir: Option<String>,
    template_id: String,
    injection_mode: Option<String>,
) -> Result<ActionResult> {
    tauri::async_runtime::spawn_blocking(move || {
        enable_instruction_inner(config_dir, &template_id, injection_mode)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("启用模板提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_enable_saved_prompt(
    config_dir: Option<String>,
    id: String,
    injection_mode: Option<String>,
) -> Result<ActionResult> {
    tauri::async_runtime::spawn_blocking(move || enable_saved_prompt_inner(config_dir, id, injection_mode))
        .await
        .map_err(|e| CodexxError::Config(format!("启用自定义提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_disable_instruction(
    config_dir: Option<String>,
    delete_file: Option<bool>,
) -> Result<ActionResult> {
    tauri::async_runtime::spawn_blocking(move || disable_instruction_inner(config_dir, delete_file))
        .await
        .map_err(|e| CodexxError::Config(format!("禁用指令提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_disable_external_instruction(config_dir: Option<String>) -> Result<ActionResult> {
    tauri::async_runtime::spawn_blocking(move || disable_external_instruction_inner(config_dir))
        .await
        .map_err(|e| CodexxError::Config(format!("禁用外部提示词失败: {e}")))?
}

// ---------------------------------------------------------------------------
// 内置模板刷新 / 备份还原（Codex/Claude 共用）
// ---------------------------------------------------------------------------

#[tauri::command]
pub(crate) async fn dc_refresh_builtin_prompts(
    config_dir: Option<String>,
) -> Result<Vec<crate::devconduit::prompts::BuiltinPromptStatus>> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::devconduit::prompts::catalog::refresh_builtin_prompts_with_active(|| None)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("刷新内置模板失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_list_prompt_backups(
    engine: String,
    config_dir: Option<String>,
) -> Result<Vec<crate::devconduit::prompt_backups::PromptBackupEntry>> {
    tauri::async_runtime::spawn_blocking(move || {
        let codex_dir = if engine.trim().eq_ignore_ascii_case("codex") {
            Some(resolve_codex_dir(config_dir)?)
        } else {
            None
        };
        crate::devconduit::prompt_backups::list_prompt_backups(
            engine.trim(),
            codex_dir.as_deref(),
        )
    })
    .await
    .map_err(|e| CodexxError::Config(format!("读取备份列表失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_restore_prompt_backup(
    engine: String,
    codex_dir: Option<String>,
    backup_id: String,
) -> Result<String> {
    tauri::async_runtime::spawn_blocking(move || {
        let dir = codex_dir
            .as_deref()
            .map(|value| PathBuf::from(value.trim()));
        crate::devconduit::prompt_backups::restore_prompt_backup(
            engine.trim(),
            dir.as_deref(),
            backup_id.trim(),
        )
        .map(|_| "已还原".to_string())
    })
    .await
    .map_err(|e| CodexxError::Config(format!("还原备份失败: {e}")))?
}
