use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum PromptInjectionMode {
    Append,
    Replace,
}

impl PromptInjectionMode {
    pub(crate) fn parse(value: Option<&str>) -> crate::devconduit::error::Result<Self> {
        match value
            .unwrap_or("replace")
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "append" | "agents" => Ok(Self::Append),
            "replace" | "model" => Ok(Self::Replace),
            other => Err(crate::devconduit::error::CodexxError::Config(format!(
                "未知提示词注入模式: {other}"
            ))),
        }
    }

    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Append => "append",
            Self::Replace => "replace",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedPrompt {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) filename: String,
    pub(crate) content: String,
    /// 配套 rules 文件内容（可选）。启用时写入 ~/.codex/rules/<id>.rules
    #[serde(default)]
    pub(crate) rules_content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuiltinPromptStatus {
    pub(crate) id: String,
    pub(crate) filename: String,
    pub(crate) title: String,
    pub(crate) has_rules: bool,
    pub(crate) subtitle: String,
    pub(crate) badge: String,
    pub(crate) source_url: String,
    pub(crate) cached: bool,
    pub(crate) updated: bool,
    pub(crate) content_source: String,
    pub(crate) sync_issue: Option<String>,
    pub(crate) checked_at: Option<String>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BundledPromptMeta {
    pub(crate) id: &'static str,
    pub(crate) filename: &'static str,
    pub(crate) title: &'static str,
    pub(crate) subtitle: &'static str,
    pub(crate) badge: &'static str,
    pub(crate) content: &'static str,
    /// 配套 rules（启用时写 ~/.codex/rules/dc-<id>.rules）
    pub(crate) rules_content: Option<&'static str>,
}

#[derive(Debug, Clone)]
pub(crate) struct CachedBuiltinPrompt {
    pub(crate) id: String,
    pub(crate) filename: String,
    pub(crate) source_url: String,
    pub(crate) content: String,
    pub(crate) checked_at: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct GithubContentEntry {
    pub(crate) name: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) download_url: Option<String>,
}
