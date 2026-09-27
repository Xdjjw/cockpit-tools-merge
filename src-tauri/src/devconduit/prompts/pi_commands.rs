//! Pi 提示词注入（Tauri command 薄壳）。

use crate::devconduit::constants::LSKILL_159_ID;
use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::prompts::lskill;
use crate::devconduit::prompts::store::{
    get_saved_prompt_inner, save_prompt_inner, ENGINE_PI,
};
use crate::devconduit::prompts::SavedPrompt;
use crate::devconduit::state::{build_pi_state, PiActionResult};

fn apply_outcome(outcome: lskill::LskillApplyOutcome) -> Result<PiActionResult> {
    let state = build_pi_state()?;
    Ok(PiActionResult {
        ok: true,
        message: outcome.message,
        backup_id: outcome.backup_id,
        state,
    })
}

#[tauri::command]
pub(crate) async fn dc_pi_prompt_state() -> Result<crate::devconduit::state::PiState> {
    tauri::async_runtime::spawn_blocking(build_pi_state)
        .await
        .map_err(|e| CodexxError::Config(format!("读取 Pi 提示词状态失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_enable_pi_builtin(
    template_id: Option<String>,
    _injection_mode: Option<String>,
) -> Result<PiActionResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let resolved = template_id.unwrap_or_default();
        let id = resolved.trim();
        if !id.is_empty() && id != LSKILL_159_ID {
            return Err(CodexxError::Config(format!("未知的 Pi 内置模板: {id}")));
        }
        apply_outcome(lskill::apply_pi_lskill_159()?)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("启用 Pi 模板提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_enable_pi_saved(
    id: String,
    _injection_mode: Option<String>,
) -> Result<PiActionResult> {
    tauri::async_runtime::spawn_blocking(move || {
        let prompt = get_saved_prompt_inner(id.trim(), ENGINE_PI)?;
        if prompt.filename.eq_ignore_ascii_case("lskill-1.5.9.md")
            || prompt.id == LSKILL_159_ID
            || prompt.content.contains("<!-- LSKILL_OFFLINE_ROUTES -->")
        {
            return apply_outcome(lskill::apply_pi_lskill_159()?);
        }
        Err(CodexxError::Config(
            "Pi 目前只支持启用破甲 1.5.9 离线通道".to_string(),
        ))
    })
    .await
    .map_err(|e| CodexxError::Config(format!("启用 Pi 自定义提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_disable_pi_instruction() -> Result<PiActionResult> {
    tauri::async_runtime::spawn_blocking(|| apply_outcome(lskill::clear_pi_lskill_159()?))
        .await
        .map_err(|e| CodexxError::Config(format!("禁用 Pi 指令提示词失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_save_pi_prompt(prompt: SavedPrompt) -> Result<SavedPrompt> {
    tauri::async_runtime::spawn_blocking(move || save_prompt_inner(prompt, ENGINE_PI))
        .await
        .map_err(|e| CodexxError::Config(format!("保存 Pi 提示词失败: {e}")))?
}
