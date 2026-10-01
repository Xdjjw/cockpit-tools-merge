// DevConduit「工坊」service 层 —— 封装 dc_* 后端命令
import { invoke } from "@tauri-apps/api/core";
import type {
  BuiltinPromptStatus,
  ClaudeActionResult,
  ClaudePromptState,
  CodexPromptState,
  McpAllEngineReport,
  McpHostDiscovery,
  McpHostInstallPlan,
  McpIntegrationInstallInput,
  PiActionResult,
  PiPromptState,
  PromptActionResult,
  PromptBackupEntry,
  SavedPrompt,
  SkillsMcpActionResult,
  SkillsMcpImportPreview,
  SkillsMcpState,
  ToolStatus,
} from "../types/workshop";

// ---------- 提示词注入 ----------

export function getCodexPromptState(configDir?: string): Promise<CodexPromptState> {
  return invoke("dc_codex_prompt_state", { configDir: configDir ?? null });
}

export function getClaudePromptState(): Promise<ClaudePromptState> {
  return invoke("dc_claude_prompt_state");
}

export function listSavedPrompts(engine?: string): Promise<SavedPrompt[]> {
  return invoke("dc_list_saved_prompts", { engine: engine ?? null });
}

export function savePrompt(prompt: SavedPrompt): Promise<SavedPrompt> {
  return invoke("dc_save_prompt", { prompt });
}

export function deleteSavedPrompt(id: string): Promise<void> {
  return invoke("dc_delete_saved_prompt", { id });
}

export function listBuiltinPrompts(): Promise<BuiltinPromptStatus[]> {
  return invoke("dc_list_builtin_prompts");
}

export function refreshBuiltinPrompts(): Promise<BuiltinPromptStatus[]> {
  return invoke("dc_refresh_builtin_prompts", { configDir: null });
}

export function enableBuiltinPrompt(
  templateId: string,
  injectionMode?: string,
  configDir?: string,
): Promise<PromptActionResult> {
  return invoke("dc_enable_builtin_prompt", {
    configDir: configDir ?? null,
    templateId,
    injectionMode: injectionMode ?? null,
  });
}

export function enableSavedPrompt(
  id: string,
  injectionMode?: string,
  configDir?: string,
): Promise<PromptActionResult> {
  return invoke("dc_enable_saved_prompt", {
    configDir: configDir ?? null,
    id,
    injectionMode: injectionMode ?? null,
  });
}

export function disableInstruction(deleteFile?: boolean, configDir?: string): Promise<PromptActionResult> {
  return invoke("dc_disable_instruction", {
    configDir: configDir ?? null,
    deleteFile: deleteFile ?? null,
  });
}

export function disableExternalInstruction(configDir?: string): Promise<PromptActionResult> {
  return invoke("dc_disable_external_instruction", { configDir: configDir ?? null });
}

export function enableClaudeBuiltin(
  templateId?: string,
  injectionMode?: string,
): Promise<ClaudeActionResult> {
  return invoke("dc_enable_claude_builtin", {
    templateId: templateId ?? null,
    injectionMode: injectionMode ?? null,
  });
}

export function enableClaudeSaved(id: string, injectionMode?: string): Promise<ClaudeActionResult> {
  return invoke("dc_enable_claude_saved", { id, injectionMode: injectionMode ?? null });
}

export function disableClaudeInstruction(): Promise<ClaudeActionResult> {
  return invoke("dc_disable_claude_instruction");
}

export function getPiPromptState(): Promise<PiPromptState> {
  return invoke("dc_pi_prompt_state");
}

export function enablePiBuiltin(
  templateId?: string,
  injectionMode?: string,
): Promise<PiActionResult> {
  return invoke("dc_enable_pi_builtin", {
    templateId: templateId ?? null,
    injectionMode: injectionMode ?? null,
  });
}

export function enablePiSaved(id: string, injectionMode?: string): Promise<PiActionResult> {
  return invoke("dc_enable_pi_saved", { id, injectionMode: injectionMode ?? null });
}

export function disablePiInstruction(): Promise<PiActionResult> {
  return invoke("dc_disable_pi_instruction");
}

export function savePiPrompt(prompt: SavedPrompt): Promise<SavedPrompt> {
  return invoke("dc_save_pi_prompt", { prompt });
}
// ---------- 备份 ----------

export function listPromptBackups(engine: string, configDir?: string): Promise<PromptBackupEntry[]> {
  return invoke("dc_list_prompt_backups", { engine, configDir: configDir ?? null });
}

export function restorePromptBackup(
  engine: string,
  backupId: string,
  configDir?: string,
): Promise<string> {
  return invoke("dc_restore_prompt_backup", {
    engine,
    codexDir: configDir ?? null,
    backupId,
  });
}

// ---------- Skills / MCP ----------

export function getSkillsMcpState(tool: string, configDir?: string): Promise<SkillsMcpState> {
  return invoke("dc_get_skills_mcp_state", { tool, configDir: configDir ?? null });
}

export function getToolStatus(tool: string, configDir?: string): Promise<ToolStatus> {
  return invoke("dc_get_tool_status", { tool, configDir: configDir ?? null });
}

export function previewToolImport(tool: string, configDir?: string): Promise<SkillsMcpImportPreview> {
  return invoke("dc_preview_tool_import", { tool, configDir: configDir ?? null });
}

export function importExistingSkills(tool: string, configDir?: string): Promise<SkillsMcpActionResult> {
  return invoke("dc_import_existing_skills", { tool, configDir: configDir ?? null });
}

export function installSkillZip(
  tool: string,
  fileName: string,
  bytes: number[],
  configDir?: string,
): Promise<SkillsMcpActionResult> {
  return invoke("dc_install_skill_zip", { tool, configDir: configDir ?? null, file_name: fileName, bytes });
}

export function toggleSkill(
  tool: string,
  id: string,
  enabled: boolean,
  configDir?: string,
): Promise<SkillsMcpState> {
  return invoke("dc_toggle_skill", { tool, configDir: configDir ?? null, id, enabled });
}

export function toggleMcp(
  tool: string,
  id: string,
  enabled: boolean,
  configDir?: string,
): Promise<SkillsMcpState> {
  return invoke("dc_toggle_mcp", { tool, configDir: configDir ?? null, id, enabled });
}

export function uninstallSkill(tool: string, id: string, configDir?: string): Promise<SkillsMcpActionResult> {
  return invoke("dc_uninstall_skill", { tool, configDir: configDir ?? null, id });
}

export function uninstallMcp(tool: string, id: string, configDir?: string): Promise<SkillsMcpActionResult> {
  return invoke("dc_uninstall_mcp", { tool, configDir: configDir ?? null, id });
}

export function checkSkillUpdates(tool: string, configDir?: string): Promise<SkillsMcpState> {
  return invoke("dc_check_skill_updates", { tool, configDir: configDir ?? null });
}

export function installMcpIntegration(
  tool: string,
  input: McpIntegrationInstallInput,
  configDir?: string,
): Promise<SkillsMcpActionResult> {
  return invoke("dc_install_mcp_integration", { tool, configDir: configDir ?? null, input });
}

// FORK: 全引擎 MCP 操作 —— 安装/卸载/启停一次性作用于全部已支持引擎。
export function installMcpIntegrationAll(
  input: McpIntegrationInstallInput,
  configDir?: string,
): Promise<McpAllEngineReport[]> {
  return invoke("dc_install_mcp_integration_all", { configDir: configDir ?? null, input });
}

export function uninstallMcpAll(id: string, configDir?: string): Promise<McpAllEngineReport[]> {
  return invoke("dc_uninstall_mcp_all", { configDir: configDir ?? null, id });
}

export function toggleMcpAll(
  id: string,
  enabled: boolean,
  configDir?: string,
): Promise<McpAllEngineReport[]> {
  return invoke("dc_toggle_mcp_all", { configDir: configDir ?? null, id, enabled });
}

// FORK: 寒霜 breaker-kit 技能包一键安装（后端内置 zip，就地解压到引擎 skills 目录）。
export function installBreakerKit(tool: string, configDir?: string): Promise<SkillsMcpActionResult> {
  return invoke("dc_install_breaker_kit", { tool, configDir: configDir ?? null });
}

export function discoverMcpHosts(): Promise<McpHostDiscovery[]> {
  return invoke("dc_discover_mcp_hosts");
}

export function detectMcpHost(
  integrationId: string,
  mode?: string,
  hostPath?: string,
): Promise<McpHostInstallPlan> {
  return invoke("dc_detect_mcp_host", {
    integrationId,
    mode: mode ?? null,
    hostPath: hostPath ?? null,
  });
}

export function restoreMcpHostInstall(integrationId: string): Promise<string> {
  return invoke("dc_restore_mcp_host_install", { integrationId });
}

// ---------- 导入 DevConduit 旧库 ----------

export function importDevconduitPrompts(engineFilter?: string, dryRun?: boolean): Promise<number> {
  return invoke("dc_import_devconduit", {
    engineFilter: engineFilter ?? null,
    dryRun: dryRun ?? null,
  });
}
