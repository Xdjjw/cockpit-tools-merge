//! Skills / MCP 自动挂载（Tauri command 薄壳）。
//!
//! 复用的是 `skills_mcp/tool.rs` 的通用管道（Codex/Claude 分支），
//! 以及 `skills_mcp/catalog.rs` 的自动获取目录（Cheat Engine / x64dbg / Burp / IDA）。

use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::skills_mcp::catalog::{install_mcp_integration_inner, McpIntegrationInstallInput};
use crate::devconduit::skills_mcp::discovery::{discover_mcp_hosts_inner, McpHostDiscovery};
use crate::devconduit::skills_mcp::host_install::{
    detect_mcp_host_inner, restore_latest_mcp_host_install_inner, McpHostInstallPlan,
};
use crate::devconduit::skills_mcp::tool::{
    build_tool_state_inner, check_tool_skill_updates_inner, import_tool_resources_inner,
    install_mcp_integration_all_inner, install_tool_skill_zip_inner, preview_tool_import_inner,
    toggle_mcp_all_inner, toggle_tool_mcp_inner, toggle_tool_skill_inner,
    uninstall_mcp_all_inner, uninstall_tool_mcp_inner, uninstall_tool_skill_inner,
    McpAllEngineReport,
};
use crate::devconduit::skills_mcp::types::{SkillsMcpActionResult, SkillsMcpImportPreview, SkillsMcpState};
use crate::devconduit::tools::{ToolId, ToolStatus};

#[tauri::command]
pub(crate) async fn dc_get_skills_mcp_state(
    tool: ToolId,
    config_dir: Option<String>,
) -> Result<SkillsMcpState> {
    tauri::async_runtime::spawn_blocking(move || build_tool_state_inner(tool, config_dir))
        .await
        .map_err(|e| CodexxError::Config(format!("读取 Skills/MCP 状态失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_get_tool_status(
    tool: ToolId,
    config_dir: Option<String>,
) -> Result<ToolStatus> {
    tauri::async_runtime::spawn_blocking(move || crate::devconduit::tools::status_for_tool_wrapper(tool, config_dir))
        .await
        .map_err(|e| CodexxError::Config(format!("读取工具状态失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_preview_tool_import(
    tool: ToolId,
    config_dir: Option<String>,
) -> Result<SkillsMcpImportPreview> {
    tauri::async_runtime::spawn_blocking(move || preview_tool_import_inner(tool, config_dir))
        .await
        .map_err(|e| CodexxError::Config(format!("预览导入失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_import_existing_skills(
    tool: ToolId,
    config_dir: Option<String>,
) -> Result<SkillsMcpActionResult> {
    tauri::async_runtime::spawn_blocking(move || import_tool_resources_inner(tool, config_dir))
        .await
        .map_err(|e| CodexxError::Config(format!("导入已有 Skills 失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_install_skill_zip(
    tool: ToolId,
    config_dir: Option<String>,
    file_name: String,
    bytes: Vec<u8>,
) -> Result<SkillsMcpActionResult> {
    tauri::async_runtime::spawn_blocking(move || {
        install_tool_skill_zip_inner(tool, config_dir, file_name, bytes)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("安装 Skill ZIP 失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_toggle_skill(
    tool: ToolId,
    config_dir: Option<String>,
    id: String,
    enabled: bool,
) -> Result<SkillsMcpState> {
    tauri::async_runtime::spawn_blocking(move || {
        toggle_tool_skill_inner(tool, config_dir, id, enabled)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("切换 Skill 失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_toggle_mcp(
    tool: ToolId,
    config_dir: Option<String>,
    id: String,
    enabled: bool,
) -> Result<SkillsMcpState> {
    tauri::async_runtime::spawn_blocking(move || {
        toggle_tool_mcp_inner(tool, config_dir, id, enabled)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("切换 MCP 失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_uninstall_skill(
    tool: ToolId,
    config_dir: Option<String>,
    id: String,
) -> Result<SkillsMcpActionResult> {
    tauri::async_runtime::spawn_blocking(move || uninstall_tool_skill_inner(tool, config_dir, id))
        .await
        .map_err(|e| CodexxError::Config(format!("卸载 Skill 失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_uninstall_mcp(
    tool: ToolId,
    config_dir: Option<String>,
    id: String,
) -> Result<SkillsMcpActionResult> {
    tauri::async_runtime::spawn_blocking(move || uninstall_tool_mcp_inner(tool, config_dir, id))
        .await
        .map_err(|e| CodexxError::Config(format!("卸载 MCP 失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_check_skill_updates(
    tool: ToolId,
    config_dir: Option<String>,
) -> Result<SkillsMcpState> {
    tauri::async_runtime::spawn_blocking(move || check_tool_skill_updates_inner(tool, config_dir))
        .await
        .map_err(|e| CodexxError::Config(format!("检查 Skill 更新失败: {e}")))?
}

/// FORK: 安装内置的寒霜 breaker-kit 技能包（107 技能 + RULES + 脚本）。
#[tauri::command]
pub(crate) async fn dc_install_breaker_kit(
    tool: ToolId,
    config_dir: Option<String>,
) -> Result<SkillsMcpActionResult> {
    const KIT_ZIP: &[u8] = crate::devconduit::constants::HANSHUANG_BREAKER_KIT_ZIP_BYTES;
    tauri::async_runtime::spawn_blocking(move || {
        crate::devconduit::skills_mcp::tool::install_tool_skill_zip_inner(
            tool,
            config_dir,
            "hanshuang-breaker-kit.zip".to_string(),
            KIT_ZIP.to_vec(),
        )
    })
    .await
    .map_err(|e| CodexxError::Config(format!("安装寒霜技能包失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_install_mcp_integration(
    tool: ToolId,
    config_dir: Option<String>,
    input: McpIntegrationInstallInput,
) -> Result<SkillsMcpActionResult> {
    tauri::async_runtime::spawn_blocking(move || install_mcp_integration_inner(tool, config_dir, input))
        .await
        .map_err(|e| CodexxError::Config(format!("安装 MCP 集成失败: {e}")))?
}

/// FORK: 把同一份 MCP 集成安装到全部引擎（Codex/Claude/Grok/ZCode/Kilo/Pi）。
#[tauri::command]
pub(crate) async fn dc_install_mcp_integration_all(
    config_dir: Option<String>,
    input: McpIntegrationInstallInput,
) -> Result<Vec<McpAllEngineReport>> {
    tauri::async_runtime::spawn_blocking(move || {
        install_mcp_integration_all_inner(config_dir, &input)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("全引擎安装 MCP 集成失败: {e}")))?
}

/// FORK: 从全部引擎卸载同一个 MCP（未安装的引擎跳过）。
#[tauri::command]
pub(crate) async fn dc_uninstall_mcp_all(
    config_dir: Option<String>,
    id: String,
) -> Result<Vec<McpAllEngineReport>> {
    tauri::async_runtime::spawn_blocking(move || uninstall_mcp_all_inner(config_dir, &id))
        .await
        .map_err(|e| CodexxError::Config(format!("全引擎卸载 MCP 失败: {e}")))?
}

/// FORK: 在全部引擎统一启停同一个 MCP（未安装的引擎跳过）。
#[tauri::command]
pub(crate) async fn dc_toggle_mcp_all(
    config_dir: Option<String>,
    id: String,
    enabled: bool,
) -> Result<Vec<McpAllEngineReport>> {
    tauri::async_runtime::spawn_blocking(move || toggle_mcp_all_inner(config_dir, &id, enabled))
        .await
        .map_err(|e| CodexxError::Config(format!("全引擎切换 MCP 状态失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_discover_mcp_hosts() -> Result<Vec<McpHostDiscovery>> {
    let hosts = tauri::async_runtime::spawn_blocking(discover_mcp_hosts_inner)
        .await
        .map_err(|e| CodexxError::Config(format!("发现宿主失败: {e}")))?;
    Ok(hosts)
}

#[tauri::command]
pub(crate) async fn dc_detect_mcp_host(
    integration_id: String,
    mode: Option<String>,
    host_path: Option<String>,
) -> Result<McpHostInstallPlan> {
    tauri::async_runtime::spawn_blocking(move || {
        detect_mcp_host_inner(integration_id, mode, host_path)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("检测宿主失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_restore_mcp_host_install(integration_id: String) -> Result<String> {
    tauri::async_runtime::spawn_blocking(move || restore_latest_mcp_host_install_inner(integration_id))
        .await
        .map_err(|e| CodexxError::Config(format!("恢复宿主文件失败: {e}")))?
}

#[tauri::command]
pub(crate) async fn dc_import_devconduit(
    engine_filter: Option<String>,
    dry_run: Option<bool>,
) -> Result<u64> {
    tauri::async_runtime::spawn_blocking(move || {
        crate::devconduit::import_devconduit::import_prompts_from_devconduit(
            engine_filter,
            dry_run.unwrap_or(false),
        )
    })
    .await
    .map_err(|e| CodexxError::Config(format!("导入 DevConduit 提示词失败: {e}")))?
    .map(|(imported, _)| imported)
}
