//! 多引擎（ZCode / Grok / Kilo / Pi / Claude Runtime / Provider 管理）编译期 stub。
//!
//! 本移植只支持 Codex 与 Claude 两个渠道。`tools.rs` / `skills_mcp/tool.rs` 等
//! 通用管道仍按 6 引擎编译，因此这里用最小 stub 顶替已移除的多人引擎模块，
//! 保证编译通过；这些分支在 Codex/Claude 运行路径上永不执行。

use crate::devconduit::error::Result;
use std::path::PathBuf;

/// ZCode 相关探测符号。
pub(crate) mod zcode {
    use super::*;

    pub fn detect_zcode_version() -> Option<String> {
        None
    }

    pub fn discover_zcode_app() -> std::io::Result<PathBuf> {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "zcode not supported in this build",
        ))
    }
}

/// 当前生效的 ZCode Provider（stub，仅类型占位）。
#[derive(Debug, Clone)]
pub struct ZcodeProviderCurrent {
    pub model: String,
    pub provider_name: String,
    pub provider_id: String,
}

/// Provider 管理符号（本移植不做 Provider 管理，仅占位）。
pub(crate) mod providers {
    use super::*;

    pub fn current_zcode_provider_inner() -> Result<Option<ZcodeProviderCurrent>> {
        Ok(None)
    }
}

/// Pi 桥接文件（本移植不做 Pi，仅为通用管道编译占位）。
pub(crate) mod pi {
    use super::*;

    #[derive(Debug, Clone, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PiMcpAdapterInstall {
        pub adapter_id: String,
    }

    pub fn mcp_config_path() -> Result<PathBuf> {
        Ok(crate::devconduit::paths::home_dir()?.join(".pi").join("agent").join("mcp.json"))
    }

    pub fn mcp_adapter_installed() -> Result<bool> {
        Ok(false)
    }

    pub fn ensure_mcp_adapter_installed() -> Result<PiMcpAdapterInstall> {
        Ok(PiMcpAdapterInstall {
            adapter_id: "pi-mcp-adapter@2.21.0".to_string(),
        })
    }

    pub fn rollback_mcp_adapter_install(
        _install: &PiMcpAdapterInstall,
    ) -> Result<()> {
        Ok(())
    }
}

/// cc-switch 状态库（stub，仅占位）。
pub(crate) mod ccswitch {
    use super::*;

    pub fn default_ccswitch_db_path() -> Result<PathBuf> {
        Ok(PathBuf::new())
    }
}
