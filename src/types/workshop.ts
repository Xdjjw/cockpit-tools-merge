// DevConduit「工坊」领域类型（对齐后端 serde camelCase）

export type WorkshopEngine = "codex" | "claude" | "pi";
export interface SavedPrompt {
  id: string;
  title: string;
  filename: string;
  content: string;
  /** 配套 rules 文件内容（启用时写 ~/.codex/rules/dc-<id>.rules） */
  rulesContent?: string | null;
}

export interface BuiltinPromptStatus {
  id: string;
  filename: string;
  title: string;
  subtitle: string;
  badge: string;
  hasRules: boolean;
  sourceUrl: string;
  cached: boolean;
  updated: boolean;
  contentSource: string;
  syncIssue: string | null;
  checkedAt: string | null;
  message: string;
}

export interface CodexProviderSummary {
  id: string;
  name: string | null;
  baseUrl: string | null;
  wireApi: string | null;
  requiresOpenaiAuth: boolean | null;
  isCurrent: boolean;
}

export interface CodexPromptState {
  codexDir: string;
  configPath: string;
  authPath: string;
  configExists: boolean;
  authExists: boolean;
  officialAuthAvailable: boolean;
  model: string | null;
  modelProvider: string | null;
  instructionFile: string | null;
  instructionEnabled: boolean;
  instructionStatus: string;
  inactiveInstructionFile: string | null;
  instructionInjectionMode: string | null;
  instructionTemplateKey: string | null;
  agentsPath: string;
  providers: CodexProviderSummary[];
  configPreview: string;
  authPreview: unknown;
}

export interface PromptActionResult {
  ok: boolean;
  message: string;
  backupId: string | null;
  state: CodexPromptState;
}

export interface ClaudePromptState {
  claudeDir: string;
  memoryPath: string;
  memoryExists: boolean;
  instructionEnabled: boolean;
  instructionInjectionMode: string | null;
  instructionTemplateKey: string | null;
  activeInstructionTitle: string | null;
}

export interface ClaudeActionResult {
  ok: boolean;
  message: string;
  backupId: string | null;
  state: ClaudePromptState;
}

export interface PiPromptState {
  piDir: string;
  agentsPath: string;
  agentsExists: boolean;
  instructionEnabled: boolean;
  instructionInjectionMode: string | null;
  instructionTemplateKey: string | null;
  activeInstructionTitle: string | null;
}

export interface PiActionResult {
  ok: boolean;
  message: string;
  backupId: string | null;
  state: PiPromptState;
}
export interface PromptBackupEntry {
  id: string;
  engine: string;
  action: string;
  createdAt: string;
  path: string;
  scope: string | null;
  injectionMode: string | null;
  fileCount: number;
}

export interface ManagedSkill {
  id: string;
  name: string;
  description: string | null;
  directory: string;
  enabled: boolean;
  source: string;
  path: string;
  contentHash: string | null;
  updateStatus: string;
  installed: boolean;
  readOnly: boolean;
}

export interface ManagedMcpServer {
  id: string;
  name: string;
  transport: string;
  enabled: boolean;
  source: string;
  summary: string;
  command: string | null;
  url: string | null;
  configJson: unknown;
  installed: boolean;
}

export interface SkillsMcpState {
  tool: string;
  toolLabel: string;
  toolDir: string;
  skillsDir: string;
  configPath: string;
  codexDir: string;
  codexSkillsDir: string;
  disabledSkillsDir: string;
  mcpAdapterInstalled: boolean | null;
  skills: ManagedSkill[];
  mcpServers: ManagedMcpServer[];
  warnings: string[];
}

export interface SkillsMcpActionResult {
  importedSkills: number;
  importedMcp: number;
  message: string;
  state: SkillsMcpState;
}

export interface SkillsMcpImportPreview {
  skills: ManagedSkill[];
  mcpServers: ManagedMcpServer[];
  warnings: string[];
}

export interface McpHostCandidate {
  integrationId: string;
  path: string;
  label: string;
  automatic: boolean;
}

export interface McpHostDiscovery {
  integrationId: string;
  candidates: McpHostCandidate[];
}

export interface McpHostInstallTarget {
  hostPath: string;
  relativePath: string;
  installed: boolean;
  isBackupPresent: boolean;
}

export interface McpHostInstallPlan {
  integrationId: string;
  status: string;
  hostName: string;
  hostPath: string | null;
  targets: McpHostInstallTarget[];
  canRestore: boolean;
  message: string;
  nextStep: string | null;
}

export interface McpIntegrationInstallInput {
  integrationId: string;
  sourcePath?: string | null;
  hostPath?: string | null;
  command?: string | null;
  endpoint?: string | null;
  mode?: string | null;
  autoInstall?: boolean | null;
  sourceMode?: string | null;
}

export interface McpAllEngineReport {
  tool: string;
  toolLabel: string;
  ok: boolean;
  message: string;
}

export interface ToolStatus {
  id: string;
  label: string;
  installed: boolean;
  version: string | null;
  homeDir: string;
  configPath: string;
  configFormat: string;
  configExists: boolean;
  authPath: string | null;
  authExists: boolean;
  instructionPath: string;
  nativeInstructionPath: string;
  diagnosticPath: string | null;
  instructionExists: boolean;
  instructionEnabled: boolean;
  model: string | null;
  provider: string | null;
  providerId: string | null;
  notice: string | null;
}
