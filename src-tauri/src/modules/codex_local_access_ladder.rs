// 破甲阶梯与工具路由的持久化状态。
//
// 这两个模块的状态文件由 Rust 侧写、Go sidecar 侧每 3 秒热加载读取,
// 所以字段名必须与 Go 侧的 JSON tag 严格对齐:
//   ladder-state.json      <-> Go ladder.go 的 ladderState
//   toolrouter-state.json  <-> Go toolrouter.go 的 toolRouterState
//
// 通过 include! 与 codex_local_access 共享同一模块作用域, 因此这里
// 直接使用 foundation 分片里已导入的 fs / Path / PathBuf / serde 等。

const CODEX_LOCAL_ACCESS_SIDECAR_LADDER_STATE_FILE: &str = "ladder-state.json";
const CODEX_LOCAL_ACCESS_SIDECAR_TOOLROUTER_STATE_FILE: &str = "toolrouter-state.json";

/// 破甲阶梯的持久化状态。字段名与 Go 侧 `ladderState` 对齐。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLocalAccessLadderState {
    pub enabled: bool,
    /// 当前生效层 0-4。0 为直通(原文保真), 4 为目标抽象化。
    pub level: u8,
    /// 非流式被拒后是否静默升级重发。
    pub auto_retry: bool,
    /// 单次请求允许的升级次数上限。
    pub max_retries: u8,
    /// 阶梯顶端; 到顶后不再升级, 如实报错。
    pub max_level: u8,
}

impl Default for CodexLocalAccessLadderState {
    fn default() -> Self {
        Self {
            enabled: true,
            // 从直通层起步: 先试保真度最高的一层, 被拒才逐级升级。
            level: 0,
            auto_retry: true,
            max_retries: 4,
            max_level: 4,
        }
    }
}

fn sidecar_ladder_state_path(base_dir: &Path) -> PathBuf {
    base_dir.join(CODEX_LOCAL_ACCESS_SIDECAR_LADDER_STATE_FILE)
}

/// 读取阶梯状态。文件不存在时返回默认值(启用、L0 直通、允许自动重试)。
pub fn read_local_access_ladder_state() -> Result<CodexLocalAccessLadderState, String> {
    let base_dir = local_access_sidecar_dir()?;
    let path = sidecar_ladder_state_path(&base_dir);
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(CodexLocalAccessLadderState::default());
    };
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(CodexLocalAccessLadderState::default());
    }

    // 只解析文件里实际存在的字段: 升级层是 sidecar 自己写回的,
    // 用户配置字段缺失时要用默认值补齐, 不能被解析失败整体丢掉。
    let raw: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|error| format!("解析破甲阶梯状态失败: {}", error))?;
    let defaults = CodexLocalAccessLadderState::default();

    Ok(CodexLocalAccessLadderState {
        enabled: raw
            .get("enabled")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(defaults.enabled),
        level: raw
            .get("level")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value.min(defaults.max_level as u64) as u8)
            .unwrap_or(defaults.level),
        auto_retry: raw
            .get("autoRetry")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(defaults.auto_retry),
        max_retries: raw
            .get("maxRetries")
            .and_then(serde_json::Value::as_u64)
            .map(|value| value.min(defaults.max_level as u64) as u8)
            .unwrap_or(defaults.max_retries),
        max_level: defaults.max_level,
    })
}

/// 写入阶梯状态。sidecar 侧有 3 秒热加载轮询, 保存后无需重启即可生效。
pub fn write_local_access_ladder_state(
    state: &CodexLocalAccessLadderState,
) -> Result<CodexLocalAccessLadderState, String> {
    let base_dir = local_access_sidecar_dir()?;
    fs::create_dir_all(&base_dir)
        .map_err(|error| format!("创建 API 服务 sidecar 目录失败: {}", error))?;

    let clamped = CodexLocalAccessLadderState {
        enabled: state.enabled,
        level: state.level.min(state.max_level),
        auto_retry: state.auto_retry,
        max_retries: state.max_retries.min(state.max_level),
        max_level: state.max_level,
    };

    // sidecar 读的是 camelCase 的 autoRetry/maxRetries, 不能直接用结构体默认命名。
    let payload = serde_json::json!({
        "enabled": clamped.enabled,
        "level": clamped.level,
        "autoRetry": clamped.auto_retry,
        "maxRetries": clamped.max_retries,
    });
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|error| format!("序列化破甲阶梯状态失败: {}", error))?;

    let path = sidecar_ladder_state_path(&base_dir);
    write_string_atomic(&path, &content)
        .map_err(|error| format!("写入破甲阶梯状态失败: {}", error))?;

    Ok(clamped)
}

/// 供前端面板查询当前阶梯状态。
pub fn local_access_ladder_state() -> Result<CodexLocalAccessLadderState, String> {
    read_local_access_ladder_state()
}

/// 更新阶梯的启用状态与自动重试策略。
pub async fn update_local_access_ladder(
    enabled: bool,
    auto_retry: bool,
    max_retries: u8,
) -> Result<CodexLocalAccessLadderState, String> {
    let current = read_local_access_ladder_state()?;
    let next = CodexLocalAccessLadderState {
        enabled,
        level: current.level,
        auto_retry,
        max_retries,
        max_level: current.max_level,
    };
    write_local_access_ladder_state(&next)
}

/// 手动指定当前层, 用于在前端面板上直接锁定某一层试效果。
pub async fn update_local_access_ladder_level(
    level: u8,
) -> Result<CodexLocalAccessLadderState, String> {
    let current = read_local_access_ladder_state()?;
    let next = CodexLocalAccessLadderState {
        level,
        ..current
    };
    write_local_access_ladder_state(&next)
}

/// 工具路由档案。字段名与 Go 侧 `toolRouteProfile` 的 JSON tag 对齐。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLocalAccessToolProfile {
    pub id: String,
    pub label: String,
    /// 空数组表示全部放行(不裁剪)。这是保守默认, 不要改成白名单。
    #[serde(default)]
    pub allow_namespaces: Vec<String>,
    /// 优先于白名单, 命中即移除。
    #[serde(default)]
    pub deny_namespaces: Vec<String>,
    /// 该任务类型要用的模型; 空表示不改模型。
    #[serde(default)]
    pub model: String,
}

/// 工具路由规则。分类用正则, weight 决定同类内的相对强度。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLocalAccessToolRule {
    pub class: String,
    pub pattern: String,
    #[serde(default)]
    pub weight: i32,
}

/// 工具路由状态。配置面(可编辑)与统计(只读, 来自 sidecar)分开。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexLocalAccessToolRouterState {
    pub enabled: bool,
    /// 独立开关: 裁剪和换模型可以分开用。
    pub model_routing: bool,
    /// 分类不出结果时用的档案 ID。这个档案必须保守(不裁剪)。
    pub default_profile: String,
    pub profiles: Vec<CodexLocalAccessToolProfile>,
    pub rules: Vec<CodexLocalAccessToolRule>,
}

fn sidecar_tool_router_state_path(base_dir: &Path) -> PathBuf {
    base_dir.join(CODEX_LOCAL_ACCESS_SIDECAR_TOOLROUTER_STATE_FILE)
}

/// 读取工具路由配置。文件不存在时返回 None, 由前端提示"尚未初始化"。
///
/// 与 ladder 不同: ladder 的状态文件由 sidecar 自动写回, 这里必须由面板显式
/// 落一次盘才会存在。所以读不到是正常状态, 不该报错。
pub fn read_local_access_tool_router_state(
) -> Result<Option<CodexLocalAccessToolRouterState>, String> {
    let base_dir = local_access_sidecar_dir()?;
    let path = sidecar_tool_router_state_path(&base_dir);
    let Ok(content) = fs::read_to_string(&path) else {
        return Ok(None);
    };
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let state: CodexLocalAccessToolRouterState = serde_json::from_str(trimmed)
        .map_err(|error| format!("解析工具路由配置失败: {}", error))?;
    Ok(Some(state))
}

/// 写入工具路由配置。sidecar 有 3 秒热加载, 保存后无需重启。
pub fn write_local_access_tool_router_state(
    state: &CodexLocalAccessToolRouterState,
) -> Result<CodexLocalAccessToolRouterState, String> {
    let base_dir = local_access_sidecar_dir()?;
    fs::create_dir_all(&base_dir)
        .map_err(|error| format!("创建 API 服务 sidecar 目录失败: {}", error))?;

    let content = serde_json::to_string_pretty(state)
        .map_err(|error| format!("序列化工具路由配置失败: {}", error))?;
    let path = sidecar_tool_router_state_path(&base_dir);
    write_string_atomic(&path, &content)
        .map_err(|error| format!("写入工具路由配置失败: {}", error))?;

    Ok(state.clone())
}

/// 供前端面板查询当前配置。
pub fn local_access_tool_router_state(
) -> Result<Option<CodexLocalAccessToolRouterState>, String> {
    read_local_access_tool_router_state()
}

/// 保存整份配置(面板编辑后提交)。
pub async fn update_local_access_tool_router(
    state: CodexLocalAccessToolRouterState,
) -> Result<CodexLocalAccessToolRouterState, String> {
    write_local_access_tool_router_state(&state)
}

/// 只切开关, 保留档案与规则。
pub async fn update_local_access_tool_router_enabled(
    enabled: bool,
) -> Result<CodexLocalAccessToolRouterState, String> {
    let current = read_local_access_tool_router_state()?
        .ok_or_else(|| "工具路由配置尚未初始化, 请先在面板中保存一次配置".to_string())?;
    let next = CodexLocalAccessToolRouterState { enabled, ..current };
    write_local_access_tool_router_state(&next)
}
