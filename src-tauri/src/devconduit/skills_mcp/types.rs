use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use serde_json::Value;

/// Resource targets supported by the Skills/MCP manager.
///
/// Pi and Kilo joined `ToolId` as first-class tools, so this is now a plain
/// alias kept for the pre-merge Skills/MCP call sites.
pub(crate) type ResourceToolId = crate::devconduit::tools::ToolId;

#[derive(Debug, Clone)]
pub(crate) struct ManagedMcpServer {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) transport: String,
    pub(crate) enabled: bool,
    pub(crate) source: String,
    pub(crate) summary: String,
    pub(crate) command: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) config_json: Value,
    pub(crate) installed: bool,
}

impl Serialize for ManagedMcpServer {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut config = self.config_json.clone();
        crate::devconduit::tools::redact_json_value(&mut config);
        let mut state = serializer.serialize_struct("ManagedMcpServer", 10)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("transport", &self.transport)?;
        state.serialize_field("enabled", &self.enabled)?;
        state.serialize_field("source", &self.source)?;
        state.serialize_field("summary", &self.summary)?;
        state.serialize_field("command", &self.command)?;
        state.serialize_field("url", &self.url)?;
        state.serialize_field("configJson", &config)?;
        state.serialize_field("installed", &self.installed)?;
        state.end()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManagedSkill {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) directory: String,
    pub(crate) enabled: bool,
    pub(crate) source: String,
    pub(crate) path: String,
    pub(crate) content_hash: Option<String>,
    pub(crate) update_status: String,
    pub(crate) installed: bool,
    pub(crate) read_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillsMcpState {
    pub(crate) tool: ResourceToolId,
    pub(crate) tool_label: String,
    pub(crate) tool_dir: String,
    pub(crate) skills_dir: String,
    pub(crate) config_path: String,
    pub(crate) codex_dir: String,
    pub(crate) codex_skills_dir: String,
    pub(crate) disabled_skills_dir: String,
    pub(crate) mcp_adapter_installed: Option<bool>,
    pub(crate) skills: Vec<ManagedSkill>,
    pub(crate) mcp_servers: Vec<ManagedMcpServer>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillsMcpActionResult {
    pub(crate) imported_skills: usize,
    pub(crate) imported_mcp: usize,
    pub(crate) message: String,
    pub(crate) state: SkillsMcpState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillsMcpImportPreview {
    pub(crate) skills: Vec<ManagedSkill>,
    pub(crate) mcp_servers: Vec<ManagedMcpServer>,
    pub(crate) warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CcSwitchSkillMeta {
    pub(super) repo_owner: String,
    pub(super) repo_name: String,
    pub(super) repo_branch: String,
    pub(super) content_hash: Option<String>,
}
