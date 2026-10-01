export type CodexLocalAccessAddressKind = "local" | "lan";
export type CodexLocalAccessScope = "localhost" | "lan";
export type CodexLocalAccessClientBaseUrlHost = "localhost" | "127.0.0.1";
export type CodexLocalAccessImageGenerationMode =
  "enabled" | "images_only" | "disabled";
export type CodexLocalAccessGatewayMode = "legacy" | "sidecar";
export type CodexLocalAccessRequestKind =
  "text" | "image_generation" | "image_edit" | "other";
export type CodexLocalAccessImageGenerationStatus =
  "unknown" | "available" | "unavailable" | "disabled";
export type CodexLocalAccessImageGenerationPolicy =
  | "inherit"
  | "enabled"
  | "disabled";

export type CodexLocalAccessRoutingStrategy =
  | "auto"
  | "random"
  | "single_account"
  | "quota_high_first"
  | "quota_low_first"
  | "plan_high_first"
  | "plan_low_first"
  | "expiry_soon_first"
  | "custom";

export interface CodexLocalAccessCustomRoutingRule {
  accountId: string;
  priority: number;
  weight: number;
  isBackup: boolean;
  isPreferred: boolean;
}

export interface CodexLocalAccessOAuthQuotaReserve {
  hourlyPercent: number;
  weeklyPercent: number;
}

export interface CodexLocalAccessAccountModelRule {
  accountId: string;
  excludedModels: string[];
}

export interface CodexLocalAccessModelAlias {
  sourceModel: string;
  alias: string;
  fork: boolean;
}

export interface CodexLocalAccessModelPricing {
  modelId: string;
  longContextThresholdTokens?: number | null;
  inputUsdPerMillion: number;
  outputUsdPerMillion: number;
  cachedInputUsdPerMillion?: number | null;
  standardLongInputUsdPerMillion?: number | null;
  standardLongOutputUsdPerMillion?: number | null;
  standardLongCachedInputUsdPerMillion?: number | null;
  priorityInputUsdPerMillion?: number | null;
  priorityOutputUsdPerMillion?: number | null;
  priorityCachedInputUsdPerMillion?: number | null;
  priorityLongInputUsdPerMillion?: number | null;
  priorityLongOutputUsdPerMillion?: number | null;
  priorityLongCachedInputUsdPerMillion?: number | null;
}

export interface CodexLocalAccessApiKey {
  id: string;
  label: string;
  key: string;
  providerGateway?: unknown | null;
  inheritAccountPool?: boolean;
  accountIds?: string[];
  priorityAccountIds?: string[];
  modelPrefix?: string | null;
  allowedModels: string[];
  excludedModels: string[];
  tokenLimit?: number | null;
  tokenUsed: number;
  enabled: boolean;
  createdAt: number;
  updatedAt: number;
  lastUsedAt?: number | null;
}

export interface CodexLocalAccessTimeouts {
  sidecarStreamOpenTimeoutMs: number;
  sidecarStreamIdleTimeoutMs: number;
  sidecarImageStreamOpenTimeoutMs: number;
  sidecarImageStreamIdleTimeoutMs: number;
  sidecarStreamOpenMaxAttempts: number;
  sidecarStreamKeepaliveSeconds: number;
  websocketConnectTimeoutMs: number;
  websocketInitialMessageTimeoutMs: number;
  websocketIdleTimeoutMs: number;
  websocketHeartbeatIntervalMs: number;
  upstreamSendRetryAttempts: number;
  upstreamSendRetryBaseDelayMs: number;
  upstreamSendRetryMaxDelayMs: number;
  singleAccountStatusRetryAttempts: number;
  singleAccountStatusRetryBaseDelayMs: number;
  singleAccountStatusRetryMaxDelayMs: number;
  sidecarStreamingBootstrapRetries: number;
}

export interface CodexLocalAccessTimeoutPreset {
  id: string;
  name: string;
  timeouts: CodexLocalAccessTimeouts;
  createdAt: number;
  updatedAt: number;
}

export interface CodexLocalAccessCollection {
  enabled: boolean;
  port: number;
  apiKey: string;
  apiKeys: CodexLocalAccessApiKey[];
  accessScope: CodexLocalAccessScope;
  clientBaseUrlHost: CodexLocalAccessClientBaseUrlHost;
  imageGenerationMode: CodexLocalAccessImageGenerationMode;
  imageGenerationModel: string;
  imageGenerationAccountPolicies: Record<
    string,
    CodexLocalAccessImageGenerationPolicy
  >;
  /** 生图转发账号池：生图与图片编辑请求只交给这些 OAuth 账号执行。 */
  imageGenerationAccountIds?: string[];
  gatewayMode: CodexLocalAccessGatewayMode;
  upstreamProxyUrl?: string | null;
  routingStrategy: CodexLocalAccessRoutingStrategy;
  customRoutingRules: CodexLocalAccessCustomRoutingRule[];
  accountModelRules: CodexLocalAccessAccountModelRule[];
  modelAliases: CodexLocalAccessModelAlias[];
  modelPricingVersion: number;
  modelPricings: CodexLocalAccessModelPricing[];
  debugLogs: boolean;
  immediateSseResponse: boolean;
  maxConcurrentImageRequests: number;
  maxAccountConcurrency: number;
  accountConcurrencyWaitMs: number;
  excludedModels: string[];
  sessionAffinity: boolean;
  sessionAffinityTtlMs: number;
  sessionAffinityDefaultEnabledMigrated?: boolean;
  responsesWebsocketsEnabled: boolean;
  maxRetryCredentials: number;
  maxRetryIntervalMs: number;
  timeouts: CodexLocalAccessTimeouts;
  activeTimeoutPresetId: string;
  timeoutPresets: CodexLocalAccessTimeoutPreset[];
  disableCooling: boolean;
  restrictFreeAccounts: boolean;
  boundOauthAccountId?: string | null;
  boundOauthQuotaReserve?: CodexLocalAccessOAuthQuotaReserve | null;
  accountIds: string[];
  createdAt: number;
  updatedAt: number;
}

export interface CodexLocalAccessUsageStats {
  requestCount: number;
  successCount: number;
  failureCount: number;
  clientCanceledCount: number;
  upstreamResponseFailedCount: number;
  streamIncompleteCount: number;
  totalLatencyMs: number;
  textRequestCount: number;
  imageRequestCount: number;
  imageGenerationRequestCount: number;
  imageEditRequestCount: number;
  imageGenerationCapabilityFailureCount: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  cachedTokens: number;
  reasoningTokens: number;
  estimatedCostUsd: number;
}

export interface CodexLocalAccessAccountStats {
  accountId: string;
  email: string;
  usage: CodexLocalAccessUsageStats;
  updatedAt: number;
}

export interface CodexLocalAccessModelStats {
  modelId: string;
  usage: CodexLocalAccessUsageStats;
  updatedAt: number;
}

export interface CodexLocalAccessApiKeyStats {
  apiKeyId: string;
  label: string;
  usage: CodexLocalAccessUsageStats;
  updatedAt: number;
}

export interface CodexLocalAccessStatsWindow {
  since: number;
  updatedAt: number;
  totals: CodexLocalAccessUsageStats;
  accounts: CodexLocalAccessAccountStats[];
  models: CodexLocalAccessModelStats[];
  apiKeys: CodexLocalAccessApiKeyStats[];
  trend: CodexLocalAccessUsageTrendPoint[];
  trendHourly: boolean;
}

export interface CodexLocalAccessUsageTrendPoint {
  bucketStart: number;
  usage: CodexLocalAccessUsageStats;
}

export interface CodexLocalAccessAccountWindowQuery {
  accountId: string;
  windowKey: string;
  startAt: number;
  endAt: number;
}

export interface CodexLocalAccessAccountWindowStats {
  accountId: string;
  windowKey: string;
  requestCount: number;
  inputTokens: number;
  cachedTokens: number;
  outputTokens: number;
  totalTokens: number;
  estimatedCostUsd: number;
}

export interface CodexTokenInputBreakdown {
  total_tokens: number;
  uncached_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
}

export interface CodexTokenOutputBreakdown {
  total_tokens: number;
  non_reasoning_tokens: number;
  reasoning_tokens: number;
}

export interface CodexTokenBreakdown {
  schema_version: number;
  quality: string;
  total_tokens: number;
  input: CodexTokenInputBreakdown;
  output: CodexTokenOutputBreakdown;
  unclassified_tokens: number;
}

export interface CodexLocalAccessProxyRoute {
  kind: "node" | "proxy" | "direct" | "unknown";
  name: string;
}

export interface CodexLocalAccessUsageEvent {
  timestamp: number;
  requestId: string;
  accountId: string;
  email: string;
  /** 请求执行时记录的代理快照；旧日志不按当前账号绑定回填。 */
  proxyRoute?: CodexLocalAccessProxyRoute | null;
  apiKeyId: string;
  apiKeyLabel: string;
  /** 多开实例目录 ID（x-cockpit-instance-id） */
  clientInstanceId?: string;
  modelId: string;
  /** 客户端请求的模型（保留路由命名空间前缀）。 */
  requestedModel?: string;
  /** 实际发送给上游的模型；与请求模型相同时前端只展示一行。 */
  upstreamModel?: string;
  gatewayMode?: CodexLocalAccessGatewayMode | null;
  requestKind: CodexLocalAccessRequestKind;
  serviceTier?: string | null;
  /** Request reasoning effort (e.g. low/medium/high/xhigh/max), when present. */
  reasoningEffort?: string | null;
  success: boolean;
  httpStatus?: number | null;
  errorCategory: string;
  errorMessage: string;
  latencyMs: number;
  inputTokens: number;
  outputTokens: number;
  totalTokens: number;
  cachedTokens: number;
  reasoningTokens: number;
  tokenBreakdown?: CodexTokenBreakdown | null;
  estimatedCostUsd: number;
  modelPricingVersion: number;
  inputUsdPerMillion: number;
  outputUsdPerMillion: number;
  cachedInputUsdPerMillion?: number | null;
}

export interface CodexLocalAccessStats {
  since: number;
  updatedAt: number;
  totals: CodexLocalAccessUsageStats;
  accounts: CodexLocalAccessAccountStats[];
  models: CodexLocalAccessModelStats[];
  apiKeys: CodexLocalAccessApiKeyStats[];
  daily: CodexLocalAccessStatsWindow;
  weekly: CodexLocalAccessStatsWindow;
  monthly: CodexLocalAccessStatsWindow;
  events: CodexLocalAccessUsageEvent[];
}

export interface CodexLocalAccessUsageEventPage {
  events: CodexLocalAccessUsageEvent[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

export interface CodexLocalAccessRequestLogQuery {
  page: number;
  pageSize: number;
  statsRange?: "daily" | "weekly" | "monthly" | null;
  startAt?: number | null;
  endAt?: number | null;
  modelQuery?: string | null;
  accountQuery?: string | null;
  apiKeyQuery?: string | null;
  instanceQuery?: string | null;
  gatewayMode?: CodexLocalAccessGatewayMode | null;
  requestKind?: CodexLocalAccessRequestKind | null;
  success?: boolean | null;
  errorCategory?: string | null;
}

export interface CodexLocalAccessAccountCooldown {
  modelId: string;
  nextRetryAt: number;
  remainingMs: number;
  reason: string;
}

export interface CodexLocalAccessAccountHealth {
  accountId: string;
  email: string;
  available: boolean;
  consecutiveFailures: number;
  lastSuccessAt: number | null;
  lastFailureAt: number | null;
  lastFailureStatus: number | null;
  lastFailureCategory: string | null;
  lastFailureMessage: string | null;
  imageGenerationStatus: CodexLocalAccessImageGenerationStatus;
  imageGenerationCheckedAt: number | null;
  schedulerAvailable: boolean | null;
  schedulerReason: string | null;
  schedulerNextRetryAt: number | null;
  cooldowns: CodexLocalAccessAccountCooldown[];
}

export interface CodexLocalAccessAccountPoolHealth {
  apiKeyId: string;
  apiKeyLabel: string;
  provider: string;
  model: string;
  requestKind: string;
  errorCode: string;
  errorMessage: string;
  diagnosticAvailable: boolean;
  candidateAuths: number;
  scopedAuths: number;
  availableAuths: number;
  unavailableAuths: number;
  modelExcludedAuths: number;
  quotaReservedAuths: number;
  imagePolicyBlockedAuths: number;
  accountStatuses: CodexLocalAccessAccountPoolMemberHealth[];
  lastFailureAt: number;
}

export interface CodexLocalAccessAccountPoolMemberHealth {
  accountId: string;
  accountEmail: string;
  available: boolean;
  reasonCode: string;
  reasonMessage: string;
}

export interface CodexLocalAccessProfileAttachment {
  profileDir: string;
  attached: boolean;
  configAttached: boolean;
  authAttached: boolean;
  modelProvider: string | null;
  baseUrl: string | null;
  expectedBaseUrl: string | null;
  error: string | null;
}

export interface CodexLocalAccessQuotaReserveStatus {
  accountId: string;
  snapshotUpdatedAt: number | null;
  snapshotFresh: boolean;
  blocked: boolean;
  warning: boolean;
  effectiveWindow: "hourly" | "weekly" | null;
  effectiveRemainingPercent: number | null;
  effectiveReservePercent: number | null;
}

export interface CodexLocalAccessState {
  collection: CodexLocalAccessCollection | null;
  running: boolean;
  preparing: boolean;
  preparationTotal: number;
  preparationCompleted: number;
  refreshingAccounts: boolean;
  accountRefreshTotal: number;
  accountRefreshCompleted: number;
  defaultProfile: CodexLocalAccessProfileAttachment | null;
  apiPortUrl: string | null;
  baseUrl: string | null;
  lanBaseUrl: string | null;
  modelIds: string[];
  modelPricingPresets: CodexLocalAccessModelPricing[];
  lastError: string | null;
  memberCount: number;
  stats: CodexLocalAccessStats;
  accountHealth: CodexLocalAccessAccountHealth[];
  accountPoolHealth: CodexLocalAccessAccountPoolHealth[];
  /** 手动恢复后仍在抑制窗口内的账号：异常列表里临时隐藏这些账号的行。 */
  recoverySuppressedAccountIds?: string[];
  quotaReserveStatus: CodexLocalAccessQuotaReserveStatus | null;
  /** FORK: 破甲引擎独立开关 —— 打开后 sidecar 不依赖 API 服务集合常驻运行。 */
  engineStandaloneEnabled?: boolean;
  /** FORK: 引擎独立模式选定的上游账号。 */
  engineStandaloneAccountIds?: string[];
}

export interface CodexLocalAccessAppendAccountSkipped {
  accountId: string;
  reason:
    | "not_found"
    | "free_restricted"
    | "pending_oauth"
    | "web_session_quota_only";
}

export interface CodexLocalAccessAppendAccountsResult {
  state: CodexLocalAccessState;
  syncedAccountIds: string[];
  addedAccountIds: string[];
  skippedAccounts: CodexLocalAccessAppendAccountSkipped[];
}

export interface CodexLocalAccessTestResult {
  modelId: string | null;
  latencyMs: number | null;
  output: string | null;
  failure: CodexLocalAccessTestFailure | null;
}

export interface CodexLocalAccessTestFailure {
  title: string;
  stage: string;
  cause: string;
  suggestion: string;
  status: number | null;
  modelId: string | null;
  detail: string | null;
  gatewayOutput: string | null;
}

export interface CodexLocalAccessChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface CodexLocalAccessChatResult {
  modelId: string;
  latencyMs: number | null;
  output: string | null;
  failure: CodexLocalAccessTestFailure | null;
}

export type CodexLocalAccessChatStreamEvent =
  | {
      sessionId: string;
      type: "delta";
      content?: string;
      reasoning?: string;
    }
  | {
      sessionId: string;
      type: "done";
      modelId: string;
      latencyMs: number | null;
    }
  | {
      sessionId: string;
      type: "error";
      failure: CodexLocalAccessTestFailure;
    };

export interface CodexLocalAccessPortCleanupResult {
  killedCount: number;
  state: CodexLocalAccessState;
}

export type CodexInstanceGatewayKind =
  | "providerGateway"
  | "mixedModel"
  | "boundOauth";

export type CodexInstanceGatewayStatus =
  | "running"
  | "unreachable"
  | "portConflict"
  | "stopped"
  | "notStarted";

/** 实例级本地网关的运行态快照，仅用于只读展示。 */
export interface CodexInstanceGatewayView {
  id: string;
  kind: CodexInstanceGatewayKind;
  runtimeId: string;
  profileDir: string;
  instanceId: string;
  instanceName: string;
  isDefault: boolean;
  accountId: string | null;
  accountLabel: string | null;
  bindHost: string;
  port: number | null;
  baseUrl: string | null;
  wireApi: string | null;
  upstreamModels: string[];
  status: CodexInstanceGatewayStatus;
  managed: boolean;
  logApiKeyId: string;
  lastError: string | null;
}

// ---------- 本分支自有: 破甲阶梯 ----------

/**
 * 破甲阶梯状态。
 *
 * 与 shield 的混淆档位是两套语义: shield 是静态混淆等级, ladder 是
 * "被拒就升级并重发"的降级链(0-4)。level 由 sidecar 在命中拦截时自动升级,
 * 也会写回同一个状态文件, 所以这里展示的可能是自动升上去的层。
 */
export interface CodexLocalAccessLadderState {
  enabled: boolean;
  /** 当前生效层: 0 直通 / 1 注入指令 / 2 关键词改写 / 3 中性化重写 / 4 目标抽象化 */
  level: number;
  /** 非流式被拒后是否静默升级重发 */
  autoRetry: boolean;
  /** 单次请求允许的升级次数上限 */
  maxRetries: number;
  /** 阶梯顶端; 到顶后不再升级, 如实报错 */
  maxLevel: number;
}

// ---------- 本分支自有: 工具路由 ----------

/**
 * 工具路由档案: 某一类任务允许摆出哪些 MCP 工具, 以及用哪个模型。
 *
 * 注意 proxy 是 HTTP 中继, 它不执行工具。这里的 allow 只决定"少摆出来",
 * 不能"凭空变出来" —— 客户端没注册的 MCP 工具无法靠配置获得。
 */
export interface CodexLocalAccessToolProfile {
  id: string;
  label: string;
  /** 空数组 = 全部放行(不裁剪)。这是保守默认, 不要改成白名单。 */
  allowNamespaces: string[];
  /** 优先于白名单, 命中即移除 */
  denyNamespaces: string[];
  /** 该任务类型要用的模型; 空字符串表示不改模型 */
  model: string;
}

/** 分类规则: class 是任务类型, pattern 是正则, weight 是同类内的相对强度。 */
export interface CodexLocalAccessToolRule {
  class: string;
  pattern: string;
  weight: number;
}

/** 工具路由配置。 */
export interface CodexLocalAccessToolRouterState {
  enabled: boolean;
  /** 独立开关: 裁剪和换模型可以分开用 */
  modelRouting: boolean;
  /** 分类不出结果时用的档案 ID; 必须保守(不裁剪) */
  defaultProfile: string;
  profiles: CodexLocalAccessToolProfile[];
  rules: CodexLocalAccessToolRule[];
}
