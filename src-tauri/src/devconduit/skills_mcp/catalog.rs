use super::catalog_download::{acquire_mcp_source, AcquiredMcpSource};
use super::host_install::{apply_mcp_host_install, rollback_mcp_host_install, HostInstallReport};
use super::installations::{save_installation_manifest, McpInstallationManifest};
use super::skills::{copy_dir_recursive, read_skill_metadata, skill_directory_for_tool};
use super::tool::install_tool_mcp_config_inner;
use super::types::{ResourceToolId, SkillsMcpActionResult, SkillsMcpState};
use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::file_io::{ensure_directory, io_err};
use crate::devconduit::paths::app_home;
use crate::devconduit::platform::program_command;
use crate::devconduit::tools::ToolId;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

const MAX_COMMAND_LENGTH: usize = 1024;
const MAX_PATH_LENGTH: usize = 4096;
const MAX_ENDPOINT_LENGTH: usize = 2048;
const MAX_REPOSITORY_ZIP_BYTES: u64 = 100 * 1024 * 1024;
const MAX_REPOSITORY_FILES: usize = 20_000;

const IDA_PRO_MCP_COMMIT: &str = "0b5f7ae4026d3c770b190ca93c0692d1b0ceab22";
const CHEAT_ENGINE_MCP_COMMIT: &str = "6bd7ce90479250a9b4b75e7944df9c105ecb2572";
const X64DBG_MCP_COMMIT: &str = "e7474e57d34aacf533d29e5778361637afef936a";
const X64DBG_MCP_PLUGINS_URL: &str =
    "https://github.com/Wasdubya/x64dbgMCP/releases/download/build1.1/MCP_Plugins.zip";
const X64DBG_MCP_PLUGINS_SHA256: &str =
    "20d0c69d0b7f2d7f251e5479cf6728be8bb5da76d3e20c9e1feb28bfbc56ce3e";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpIntegrationInstallInput {
    pub(crate) integration_id: String,
    pub(crate) source_path: Option<String>,
    pub(crate) host_path: Option<String>,
    pub(crate) command: Option<String>,
    pub(crate) endpoint: Option<String>,
    pub(crate) mode: Option<String>,
    pub(crate) auto_install: Option<bool>,
    pub(crate) source_mode: Option<String>,
}

#[derive(Debug)]
struct PreparedIntegration {
    id: &'static str,
    name: &'static str,
    config: Value,
    host_paths: Vec<String>,
    managed_paths: Vec<String>,
    installed_files: Vec<String>,
    notes: Vec<String>,
}

impl PreparedIntegration {
    fn manual(id: &'static str, name: &'static str, config: Value) -> Self {
        Self {
            id,
            name,
            config,
            host_paths: Vec::new(),
            managed_paths: Vec::new(),
            installed_files: Vec::new(),
            notes: Vec::new(),
        }
    }
}

fn clean_optional(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn validate_text(value: &str, label: &str, max_length: usize) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(CodexxError::Config(format!("{label}不能为空")));
    }
    if value.len() > max_length {
        return Err(CodexxError::Config(format!("{label}过长")));
    }
    if value
        .chars()
        .any(|character| character == '\0' || character == '\r' || character == '\n')
    {
        return Err(CodexxError::Config(format!("{label}包含非法控制字符")));
    }
    Ok(value.to_string())
}

fn command_or_default(input: &McpIntegrationInstallInput, fallback: &str) -> Result<String> {
    validate_text(
        clean_optional(input.command.as_deref()).unwrap_or(fallback),
        "MCP 命令",
        MAX_COMMAND_LENGTH,
    )
}

fn source_path(input: &McpIntegrationInstallInput, label: &str) -> Result<PathBuf> {
    let raw = clean_optional(input.source_path.as_deref())
        .ok_or_else(|| CodexxError::Config(format!("请选择{label}")))?;
    let raw = validate_text(raw, label, MAX_PATH_LENGTH)?;
    let path = PathBuf::from(raw);
    if !path.is_absolute() {
        return Err(CodexxError::Config(format!("{label}必须使用绝对路径")));
    }
    if !path.exists() {
        return Err(CodexxError::Config(format!(
            "{label}不存在: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn has_file_name(path: &Path, expected: &str) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(expected))
}

fn expected_file(
    source: &Path,
    candidates: &[&str],
    expected_name: &str,
    label: &str,
) -> Result<PathBuf> {
    if source.is_file() {
        if has_file_name(source, expected_name) {
            return Ok(source.to_path_buf());
        }
        return Err(CodexxError::Config(format!(
            "{label}应选择 {expected_name}: {}",
            source.display()
        )));
    }
    for candidate in candidates {
        let path = source.join(candidate);
        if path.is_file() {
            return Ok(path);
        }
    }
    Err(CodexxError::Config(format!(
        "{label}中没有找到 {expected_name}: {}",
        source.display()
    )))
}

fn mode_or_default<'a>(
    input: &'a McpIntegrationInstallInput,
    fallback: &'a str,
    allowed: &[&str],
) -> Result<&'a str> {
    let mode = clean_optional(input.mode.as_deref()).unwrap_or(fallback);
    if allowed.contains(&mode) {
        Ok(mode)
    } else {
        Err(CodexxError::Config(format!(
            "不支持的 MCP 安装模式: {mode}"
        )))
    }
}

fn source_mode(input: &McpIntegrationInstallInput) -> Result<&str> {
    match clean_optional(input.source_mode.as_deref()).unwrap_or("manual") {
        mode @ ("managed" | "manual") => Ok(mode),
        mode => Err(CodexxError::Config(format!(
            "不支持的 MCP 文件来源模式: {mode}"
        ))),
    }
}

fn managed_acquisition_settings(
    tool: ToolId,
    input: &McpIntegrationInstallInput,
) -> Result<(String, String)> {
    match input.integration_id.trim() {
        "ida-pro-mcp" => Ok(("local".to_string(), command_or_default(input, "uv")?)),
        "cheatengine-mcp" => {
            let fallback = if cfg!(target_os = "windows") {
                "local"
            } else {
                "remote"
            };
            let mode = mode_or_default(input, fallback, &["local", "remote"])?;
            let command = command_or_default(
                input,
                if cfg!(target_os = "windows") {
                    "python"
                } else {
                    "python3"
                },
            )?;
            Ok((mode.to_string(), command))
        }
        "x64dbg-mcp" => {
            let fallback = if cfg!(target_os = "windows") {
                "local"
            } else {
                "remote"
            };
            let mode = mode_or_default(input, fallback, &["local", "remote"])?;
            let command = command_or_default(
                input,
                if cfg!(target_os = "windows") {
                    "python"
                } else {
                    "python3"
                },
            )?;
            Ok((mode.to_string(), command))
        }
        "burp-suite-mcp" => {
            let fallback = if matches!(
                tool,
                ToolId::Claude | ToolId::Zcode | ToolId::Kilo | ToolId::Pi
            ) {
                "direct"
            } else {
                "proxy"
            };
            let mode = mode_or_default(input, fallback, &["direct", "proxy"])?;
            Ok((mode.to_string(), command_or_default(input, "java")?))
        }
        integration => Err(CodexxError::Config(format!(
            "未知的 MCP 集成: {integration}"
        ))),
    }
}

fn http_endpoint(
    input: &McpIntegrationInstallInput,
    fallback: &str,
    trailing_slash: bool,
) -> Result<String> {
    let raw = clean_optional(input.endpoint.as_deref()).unwrap_or(fallback);
    let raw = validate_text(raw, "MCP 服务地址", MAX_ENDPOINT_LENGTH)?;
    let parsed = reqwest::Url::parse(&raw)
        .map_err(|error| CodexxError::Config(format!("MCP 服务地址无效: {error}")))?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(CodexxError::Config(
            "MCP 服务地址必须是完整的 http/https URL".to_string(),
        ));
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(CodexxError::Config(
            "MCP 服务地址不能包含用户名或密码".to_string(),
        ));
    }
    let mut normalized = parsed.to_string();
    if trailing_slash && !normalized.ends_with('/') {
        normalized.push('/');
    }
    Ok(normalized)
}

fn tcp_endpoint(input: &McpIntegrationInstallInput) -> Result<(String, String)> {
    let raw = clean_optional(input.endpoint.as_deref()).unwrap_or("127.0.0.1:9876");
    let raw = validate_text(raw, "TCP 桥接地址", MAX_ENDPOINT_LENGTH)?;
    let authority = raw.strip_prefix("tcp://").unwrap_or(&raw);
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let (host, suffix) = rest
            .split_once(']')
            .ok_or_else(|| CodexxError::Config("TCP IPv6 地址缺少 ]".to_string()))?;
        let port = suffix
            .strip_prefix(':')
            .ok_or_else(|| CodexxError::Config("TCP 桥接地址缺少端口".to_string()))?;
        (host, port)
    } else {
        authority
            .rsplit_once(':')
            .ok_or_else(|| CodexxError::Config("TCP 桥接地址格式应为 host:port".to_string()))?
    };
    if host.is_empty()
        || !host.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | ':')
        })
    {
        return Err(CodexxError::Config("TCP 桥接主机名无效".to_string()));
    }
    let port = port
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| CodexxError::Config("TCP 桥接端口无效".to_string()))?;
    Ok((host.to_string(), port.to_string()))
}

fn download_bytes(url: &str, label: &str, expected_sha256: Option<&str>) -> Result<Vec<u8>> {
    let response = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|error| CodexxError::Config(format!("创建下载客户端失败: {error}")))?
        .get(url)
        .header("User-Agent", "Cockpit MCP installer")
        .send()
        .map_err(|error| CodexxError::Config(format!("下载 {label} 失败: {error}")))?;
    let bytes = response
        .bytes()
        .map_err(|error| CodexxError::Config(format!("读取 {label} 失败: {error}")))?
        .to_vec();
    if bytes.len() as u64 > MAX_REPOSITORY_ZIP_BYTES {
        return Err(CodexxError::Config(format!("{label} 超过 100MB")));
    }
    if let Some(expected) = expected_sha256 {
        use sha2::{Digest, Sha256};
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != expected.to_ascii_lowercase() {
            return Err(CodexxError::Config(format!(
                "{label} 校验失败，期望 SHA-256 {expected}，实际为 {actual}"
            )));
        }
    }
    Ok(bytes)
}

fn extract_repository(bytes: &[u8], destination: &Path, label: &str) -> Result<()> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| CodexxError::Config(format!("解析 {label} ZIP 失败: {error}")))?;
    let staging = destination.with_file_name(format!(
        ".{}-staging-{}",
        destination
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("mcp"),
        std::process::id()
    ));
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|error| io_err(&staging, error))?;
    }
    ensure_directory(&staging)?;
    let result = (|| -> Result<()> {
        let mut files = 0usize;
        let mut total = 0u64;
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).map_err(|error| {
                CodexxError::Config(format!("读取 {label} ZIP 条目失败: {error}"))
            })?;
            if file.is_dir() {
                continue;
            }
            files += 1;
            if files > MAX_REPOSITORY_FILES {
                return Err(CodexxError::Config(format!("{label} 文件数量过多")));
            }
            total = total.saturating_add(file.size());
            if total > MAX_REPOSITORY_ZIP_BYTES {
                return Err(CodexxError::Config(format!("{label} 解压后超过 100MB")));
            }
            let Some(path) = file.enclosed_name().map(|path| path.to_path_buf()) else {
                continue;
            };
            let parts = path
                .to_string_lossy()
                .replace('\\', "/")
                .split('/')
                .skip(1)
                .filter(|part| !part.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            if parts.is_empty() || parts.iter().any(|part| part == "." || part == "..") {
                continue;
            }
            let output = parts
                .iter()
                .fold(staging.clone(), |path, part| path.join(part));
            if let Some(parent) = output.parent() {
                ensure_directory(parent)?;
            }
            let mut destination_file =
                fs::File::create(&output).map_err(|error| io_err(&output, error))?;
            std::io::copy(&mut file, &mut destination_file)
                .map_err(|error| io_err(&output, error))?;
        }
        if destination.exists() {
            fs::remove_dir_all(destination).map_err(|error| io_err(destination, error))?;
        }
        fs::rename(&staging, destination).map_err(|error| io_err(destination, error))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn install_repository(
    owner: &str,
    repo: &str,
    commit: &str,
    destination: &Path,
    label: &str,
) -> Result<()> {
    // Repository directories are shared by all resource targets. Reusing a
    // complete checkout prevents one target's reconfiguration from deleting
    // files another target is currently using.
    if destination.is_dir()
        && fs::read_dir(destination)
            .ok()
            .is_some_and(|mut entries| entries.next().is_some())
    {
        return Ok(());
    }
    let url = format!("https://github.com/{owner}/{repo}/archive/{commit}.zip");
    let bytes = download_bytes(&url, label, None)?;
    extract_repository(&bytes, destination, label)
}

fn run_external(
    program: &str,
    args: &[String],
    current_dir: Option<&Path>,
    label: &str,
) -> Result<()> {
    let references = args.iter().map(String::as_str).collect::<Vec<_>>();
    let mut command = program_command(Path::new(program), &references);
    if let Some(directory) = current_dir {
        command.current_dir(directory);
    }
    let output = command.output().map_err(|error| {
        CodexxError::Config(format!(
            "启动 {label} 失败，请确认 {program} 已安装: {error}"
        ))
    })?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = detail.trim();
    let detail = if detail.is_empty() {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    } else {
        detail.to_string()
    };
    Err(CodexxError::Config(format!(
        "{label} 失败（退出码 {:?}）: {}",
        output.status.code(),
        if detail.is_empty() {
            "无详细错误"
        } else {
            &detail
        }
    )))
}

fn host_path(input: &McpIntegrationInstallInput, label: &str) -> Result<PathBuf> {
    let raw = clean_optional(input.host_path.as_deref())
        .ok_or_else(|| CodexxError::Config(format!("未识别到{label}，请选择安装目录")))?;
    let raw = validate_text(raw, label, MAX_PATH_LENGTH)?;
    let path = PathBuf::from(raw);
    if !path.is_absolute() || !path.exists() {
        return Err(CodexxError::Config(format!(
            "{label}不存在: {}",
            path.display()
        )));
    }
    Ok(path)
}

fn managed_integration_dir(id: &str) -> Result<PathBuf> {
    let root = app_home()?.join("mcp-integrations");
    ensure_directory(&root)?;
    Ok(root.join(id))
}

fn install_managed_repository_skills(
    tool: ResourceToolId,
    config_dir: Option<String>,
    repository: &Path,
) -> Result<Vec<String>> {
    let source_root = repository.join("skills");
    if !source_root.is_dir() {
        return Ok(Vec::new());
    }
    let destination_root = tool.skills_dir(config_dir)?;
    ensure_directory(&destination_root)?;
    let mut installed = Vec::new();
    for entry in fs::read_dir(&source_root).map_err(|error| io_err(&source_root, error))? {
        let entry = entry.map_err(|error| io_err(&source_root, error))?;
        let source = entry.path();
        if !source.is_dir() || !source.join("SKILL.md").is_file() {
            continue;
        }
        let fallback = entry.file_name().to_string_lossy().to_string();
        let (name, _) = read_skill_metadata(&source, &fallback);
        let destination = destination_root.join(skill_directory_for_tool(tool, &name, &fallback));
        if destination.exists() {
            // Do not overwrite a skill that may have been installed or
            // edited outside this catalog. It remains available to the tool.
            continue;
        }
        copy_dir_recursive(&source, &destination)?;
        installed.push(
            destination
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&fallback)
                .to_string(),
        );
    }
    Ok(installed)
}

fn python_virtualenv(root: &Path, command: &str, requirements: Option<&Path>) -> Result<PathBuf> {
    let venv = root.join(".venv");
    let python = if cfg!(target_os = "windows") {
        venv.join("Scripts/python.exe")
    } else {
        venv.join("bin/python")
    };
    if !python.is_file() {
        run_external(
            command,
            &[
                "-m".to_string(),
                "venv".to_string(),
                venv.display().to_string(),
            ],
            Some(root),
            "Python 虚拟环境创建",
        )?;
    }
    if let Some(requirements) = requirements.filter(|path| path.is_file()) {
        run_external(
            &python.display().to_string(),
            &[
                "-m".to_string(),
                "pip".to_string(),
                "install".to_string(),
                "-r".to_string(),
                requirements.display().to_string(),
            ],
            Some(root),
            "MCP Python 依赖安装",
        )?;
    }
    Ok(python)
}

fn ida_auto_integration(input: &McpIntegrationInstallInput) -> Result<PreparedIntegration> {
    let host = host_path(input, "IDA Pro 安装目录")?;
    let (host_root, _) =
        super::discovery::validate_host_path("ida-pro-mcp", &host).ok_or_else(|| {
            CodexxError::Config(format!("不是有效的 IDA Pro 安装目录: {}", host.display()))
        })?;
    let root = managed_integration_dir("ida-pro-mcp")?;
    install_repository(
        "mrexodia",
        "ida-pro-mcp",
        IDA_PRO_MCP_COMMIT,
        &root,
        "IDA Pro MCP",
    )?;
    let python = "python";
    let activation = host_root.join("idalib/python/py-activate-idalib.py");
    run_external(
        &python,
        &[
            activation.display().to_string(),
            "-d".to_string(),
            host_root.display().to_string(),
        ],
        Some(&root),
        "IDA Pro idalib 激活",
    )?;
    let uv = clean_optional(input.command.as_deref()).unwrap_or("uv");
    run_external(
        uv,
        &[
            "sync".to_string(),
            "--project".to_string(),
            root.display().to_string(),
        ],
        Some(&root),
        "IDA Pro MCP 依赖同步",
    )?;
    Ok(PreparedIntegration {
        id: "ida-pro-mcp",
        name: "IDA Pro MCP",
        config: json!({
            "command": uv,
            "args": ["run", "--offline", "--no-sync", "--project", root.display().to_string(), "idalib-mcp", "--stdio"]
        }),
        host_paths: vec![host_root.display().to_string()],
        managed_paths: vec![root.display().to_string()],
        installed_files: Vec::new(),
        notes: vec!["IDA idalib 激活属于共享机器配置，卸载时会保留".to_string()],
    })
}

fn cheat_engine_auto_integration(
    input: &McpIntegrationInstallInput,
) -> Result<PreparedIntegration> {
    let mode = mode_or_default(input, "local", &["local", "remote"])?;
    let remote_endpoint = if mode == "remote" {
        Some(tcp_endpoint(input)?)
    } else {
        None
    };
    let host = host_path(input, "Cheat Engine 安装目录")?;
    let (host_root, _) = super::discovery::validate_host_path("cheatengine-mcp", &host)
        .ok_or_else(|| {
            CodexxError::Config(format!(
                "不是有效的 Cheat Engine 安装目录: {}",
                host.display()
            ))
        })?;
    let root = managed_integration_dir("cheatengine-mcp")?;
    install_repository(
        "miscusi-peek",
        "cheatengine-mcp-bridge",
        CHEAT_ENGINE_MCP_COMMIT,
        &root,
        "Cheat Engine MCP",
    )?;
    let server = root.join("MCP_Server/mcp_cheatengine.py");
    let lua = root.join("MCP_Server/ce_mcp_bridge.lua");
    if !server.is_file() || !lua.is_file() {
        return Err(CodexxError::Config(
            "Cheat Engine MCP 下载内容不完整".to_string(),
        ));
    }
    let python = command_or_default(input, "python")?;
    let requirements = root.join("MCP_Server/requirements.txt");
    let python = python_virtualenv(&root, &python, Some(&requirements))?;
    let autorun = host_root.join("autorun/everything-patch-ce-mcp.lua");
    let autorun_created = if autorun.is_file() {
        let existing = fs::read(&autorun).map_err(|error| io_err(&autorun, error))?;
        let expected = fs::read(&lua).map_err(|error| io_err(&lua, error))?;
        if existing != expected {
            return Err(CodexxError::Config(format!(
                "Cheat Engine autorun 已存在且不是 DevConduit 文件，已保留原文件: {}",
                autorun.display()
            )));
        }
        false
    } else {
        if let Some(parent) = autorun.parent() {
            ensure_directory(parent)?;
        }
        fs::copy(&lua, &autorun).map_err(|error| {
            CodexxError::Config(format!(
                "已下载 Cheat Engine MCP，但写入 autorun 失败（可能需要管理员权限）: {}: {error}",
                autorun.display()
            ))
        })?;
        true
    };
    let mut config = json!({
        "command": python.display().to_string(),
        "args": [server.display().to_string()]
    });
    if let Some((host, port)) = remote_endpoint {
        config["env"] = json!({"CE_MCP_TRANSPORT":"tcp","CE_MCP_HOST":host,"CE_MCP_PORT":port});
    }
    Ok(PreparedIntegration {
        id: "cheatengine-mcp",
        name: "Cheat Engine MCP Bridge",
        config,
        host_paths: vec![host_root.display().to_string()],
        managed_paths: vec![root.display().to_string()],
        installed_files: autorun_created
            .then(|| autorun.display().to_string())
            .into_iter()
            .collect(),
        notes: Vec::new(),
    })
}

fn x64dbg_plugin_dir(root: &Path, arch: &str) -> Option<PathBuf> {
    [
        root.join(format!("release/{arch}/plugins")),
        root.join(format!("{arch}/plugins")),
        root.join("plugins"),
    ]
    .into_iter()
    .find(|path| path.parent().is_some_and(Path::is_dir) || path.is_dir())
}

fn install_x64dbg_plugins(root: &Path) -> Result<Vec<PathBuf>> {
    let bytes = download_bytes(
        X64DBG_MCP_PLUGINS_URL,
        "x64dbg MCP 插件",
        Some(X64DBG_MCP_PLUGINS_SHA256),
    )?;
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| CodexxError::Config(format!("解析 x64dbg MCP 插件失败: {error}")))?;
    let mut installed = Vec::new();
    let mut existing = false;
    for (entry_name, target) in [("MCPx64dbg.dp64", "x64"), ("MCPx64dbg.dp32", "x32")] {
        let Some(directory) = x64dbg_plugin_dir(root, target) else {
            continue;
        };
        ensure_directory(&directory)?;
        let mut file = archive.by_name(entry_name).map_err(|error| {
            CodexxError::Config(format!("x64dbg 插件包缺少 {entry_name}: {error}"))
        })?;
        let destination = directory.join(entry_name);
        if destination.exists() {
            // Do not take ownership of a plugin that was installed outside
            // this application; it must survive an uninstall.
            existing = true;
            continue;
        }
        let mut output =
            fs::File::create(&destination).map_err(|error| io_err(&destination, error))?;
        std::io::copy(&mut file, &mut output).map_err(|error| io_err(&destination, error))?;
        installed.push(destination);
    }
    if installed.is_empty() && !existing {
        return Err(CodexxError::Config(
            "没有找到可写入的 x64dbg/x32dbg plugins 目录".to_string(),
        ));
    }
    Ok(installed)
}

fn x64dbg_auto_integration(input: &McpIntegrationInstallInput) -> Result<PreparedIntegration> {
    let endpoint = http_endpoint(input, "http://127.0.0.1:8888/", true)?;
    let host = host_path(input, "x64dbg 安装目录")?;
    let (host_root, _) =
        super::discovery::validate_host_path("x64dbg-mcp", &host).ok_or_else(|| {
            CodexxError::Config(format!("不是有效的 x64dbg 安装目录: {}", host.display()))
        })?;
    let root = managed_integration_dir("x64dbg-mcp")?;
    install_repository(
        "Wasdubya",
        "x64dbgMCP",
        X64DBG_MCP_COMMIT,
        &root,
        "x64dbg MCP",
    )?;
    let script = root.join("src/x64dbg.py");
    if !script.is_file() {
        return Err(CodexxError::Config("x64dbg MCP 下载内容不完整".to_string()));
    }
    let installed_plugins = install_x64dbg_plugins(&host_root)?;
    let python = command_or_default(input, "python")?;
    let python = python_virtualenv(&root, &python, None)?;
    run_external(
        &python.display().to_string(),
        &[
            "-m".to_string(),
            "pip".to_string(),
            "install".to_string(),
            "mcp".to_string(),
            "requests".to_string(),
        ],
        Some(&root),
        "x64dbg MCP 依赖安装",
    )?;
    Ok(PreparedIntegration {
        id: "x64dbg-mcp",
        name: "x64dbg MCP",
        config: json!({
            "command": python.display().to_string(),
            "args": [script.display().to_string()],
            "env": {"X64DBG_URL": endpoint}
        }),
        host_paths: vec![host_root.display().to_string()],
        managed_paths: vec![root.display().to_string()],
        installed_files: installed_plugins
            .into_iter()
            .map(|path| path.display().to_string())
            .collect(),
        notes: Vec::new(),
    })
}

fn ida_integration(input: &McpIntegrationInstallInput) -> Result<PreparedIntegration> {
    let selected = source_path(input, "IDA Pro MCP 项目目录")?;
    let root = if selected.is_file() && has_file_name(&selected, "pyproject.toml") {
        selected
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| CodexxError::Config("IDA Pro MCP 项目路径无效".to_string()))?
    } else if selected.is_dir() {
        selected
    } else {
        return Err(CodexxError::Config(
            "IDA Pro MCP 请选择项目目录或 pyproject.toml".to_string(),
        ));
    };
    let pyproject = root.join("pyproject.toml");
    let text = fs::read_to_string(&pyproject)
        .map_err(|error| crate::devconduit::file_io::io_err(&pyproject, error))?;
    let document = text
        .parse::<toml_edit::Document>()
        .map_err(|error| CodexxError::Toml {
            path: pyproject.display().to_string(),
            message: error.to_string(),
        })?;
    if document
        .get("project")
        .and_then(|item| item.get("name"))
        .and_then(toml_edit::Item::as_str)
        != Some("ida-pro-mcp")
    {
        return Err(CodexxError::Config(format!(
            "所选目录不是 mrexodia/ida-pro-mcp: {}",
            root.display()
        )));
    }
    let command = command_or_default(input, "uv")?;
    Ok(PreparedIntegration::manual(
        "ida-pro-mcp",
        "IDA Pro MCP",
        json!({
            "command": command,
            "args": [
                "run",
                "--offline",
                "--no-sync",
                "--project",
                root.display().to_string(),
                "idalib-mcp",
                "--stdio"
            ]
        }),
    ))
}

fn cheat_engine_integration(input: &McpIntegrationInstallInput) -> Result<PreparedIntegration> {
    let fallback_mode = if cfg!(target_os = "windows") {
        "local"
    } else {
        "remote"
    };
    let mode = mode_or_default(input, fallback_mode, &["local", "remote"])?;
    if mode == "local" && !cfg!(target_os = "windows") {
        return Err(CodexxError::Config(
            "Cheat Engine 本地 Named Pipe 模式仅支持 Windows，macOS 请选择远程 TCP".to_string(),
        ));
    }
    let selected = source_path(input, "Cheat Engine MCP 项目目录")?;
    let script = expected_file(
        &selected,
        &["MCP_Server/mcp_cheatengine.py", "mcp_cheatengine.py"],
        "mcp_cheatengine.py",
        "Cheat Engine MCP 项目",
    )?;
    let lua = script
        .parent()
        .map(|parent| parent.join("ce_mcp_bridge.lua"))
        .filter(|path| path.is_file())
        .ok_or_else(|| {
            CodexxError::Config(format!(
                "Cheat Engine MCP 项目缺少 ce_mcp_bridge.lua: {}",
                script.display()
            ))
        })?;
    let command = command_or_default(
        input,
        if cfg!(target_os = "windows") {
            "python"
        } else {
            "python3"
        },
    )?;
    let mut config = json!({
        "command": command,
        "args": [script.display().to_string()]
    });
    if mode == "remote" {
        let (host, port) = tcp_endpoint(input)?;
        config["env"] = json!({
            "CE_MCP_TRANSPORT": "tcp",
            "CE_MCP_HOST": host,
            "CE_MCP_PORT": port
        });
    }
    let _ = lua;
    Ok(PreparedIntegration::manual(
        "cheatengine-mcp",
        "Cheat Engine MCP Bridge",
        config,
    ))
}

fn x64dbg_integration(input: &McpIntegrationInstallInput) -> Result<PreparedIntegration> {
    let fallback_mode = if cfg!(target_os = "windows") {
        "local"
    } else {
        "remote"
    };
    let mode = mode_or_default(input, fallback_mode, &["local", "remote"])?;
    if mode == "local" && !cfg!(target_os = "windows") {
        return Err(CodexxError::Config(
            "x64dbg 本地模式仅支持 Windows，macOS 请选择远程桥接".to_string(),
        ));
    }
    let selected = source_path(input, "x64dbg MCP Python 桥接脚本")?;
    let script = expected_file(
        &selected,
        &["src/x64dbg.py", "x64dbg.py"],
        "x64dbg.py",
        "x64dbg MCP 项目",
    )?;
    let command = command_or_default(
        input,
        if cfg!(target_os = "windows") {
            "python"
        } else {
            "python3"
        },
    )?;
    let endpoint = http_endpoint(input, "http://127.0.0.1:8888/", true)?;
    Ok(PreparedIntegration::manual(
        "x64dbg-mcp",
        "x64dbg MCP",
        json!({
            "command": command,
            "args": [script.display().to_string()],
            "env": { "X64DBG_URL": endpoint }
        }),
    ))
}

fn burp_integration(
    tool: ResourceToolId,
    input: &McpIntegrationInstallInput,
) -> Result<PreparedIntegration> {
    let fallback = if matches!(
        tool,
        ResourceToolId::Claude | ResourceToolId::Zcode | ResourceToolId::Kilo | ResourceToolId::Pi
    ) {
        "direct"
    } else {
        "proxy"
    };
    let mode = mode_or_default(input, fallback, &["direct", "proxy"])?;
    if mode == "direct"
        && !matches!(
            tool,
            ResourceToolId::Claude
                | ResourceToolId::Zcode
                | ResourceToolId::Pi
                | ResourceToolId::Kilo
        )
    {
        return Err(CodexxError::Config(format!(
            "{} 不支持 Burp 的传统 SSE 直连，请使用官方 stdio 代理 mcp-proxy-all.jar",
            tool.label()
        )));
    }
    let endpoint = http_endpoint(input, "http://127.0.0.1:9876/sse", false)?;
    let config = if mode == "proxy" {
        let selected = source_path(input, "Burp MCP stdio 代理 JAR")?;
        if !selected.is_file()
            || selected
                .extension()
                .and_then(|extension| extension.to_str())
                .is_none_or(|extension| !extension.eq_ignore_ascii_case("jar"))
            || selected
                .file_name()
                .and_then(|name| name.to_str())
                .is_none_or(|name| !name.to_ascii_lowercase().contains("mcp-proxy"))
        {
            return Err(CodexxError::Config(
                "请选择 Burp MCP 扩展导出的 mcp-proxy-all.jar".to_string(),
            ));
        }
        let command = command_or_default(input, "java")?;
        json!({
            "command": command,
            "args": ["-jar", selected.display().to_string(), "--sse-url", endpoint]
        })
    } else {
        json!({ "type": "sse", "url": endpoint })
    };
    Ok(PreparedIntegration::manual(
        "burp-suite-mcp",
        "Burp Suite MCP Server",
        config,
    ))
}

fn prepare_integration(
    tool: ResourceToolId,
    input: &McpIntegrationInstallInput,
) -> Result<PreparedIntegration> {
    match input.integration_id.trim() {
        "ida-pro-mcp" => ida_integration(input),
        "cheatengine-mcp" => cheat_engine_integration(input),
        "x64dbg-mcp" => x64dbg_integration(input),
        "burp-suite-mcp" => burp_integration(tool, input),
        integration => Err(CodexxError::Config(format!(
            "未知的 MCP 集成: {integration}"
        ))),
    }
}

fn prepare_auto_integration(
    _tool: ResourceToolId,
    input: &McpIntegrationInstallInput,
) -> Result<PreparedIntegration> {
    match input.integration_id.trim() {
        "ida-pro-mcp" => ida_auto_integration(input),
        "cheatengine-mcp" => cheat_engine_auto_integration(input),
        "x64dbg-mcp" => x64dbg_auto_integration(input),
        "burp-suite-mcp" => Err(CodexxError::Config(
            "Burp Suite MCP 需要先在 Burp 中安装官方扩展，暂不自动下载扩展".to_string(),
        )),
        integration => Err(CodexxError::Config(format!(
            "未知的 MCP 集成: {integration}"
        ))),
    }
}

pub(crate) fn install_mcp_integration_inner(
    tool: ResourceToolId,
    config_dir: Option<String>,
    mut input: McpIntegrationInstallInput,
) -> Result<SkillsMcpActionResult> {
    let automatic = input.auto_install.unwrap_or(false);
    let acquired: Option<AcquiredMcpSource> = if !automatic && source_mode(&input)? == "managed" {
        let (mode, command) = managed_acquisition_settings(tool, &input)?;
        let source = acquire_mcp_source(input.integration_id.trim(), &mode, &command)?;
        input.mode = Some(mode);
        input.source_path = source
            .source_path
            .as_ref()
            .map(|path| path.display().to_string());
        if let Some(runtime_command) = source.runtime_command.as_ref() {
            input.command = Some(runtime_command.display().to_string());
        }
        Some(source)
    } else {
        None
    };
    let prepared = if automatic {
        prepare_auto_integration(tool, &input)?
    } else {
        prepare_integration(tool, &input)?
    };
    let host_report: Option<HostInstallReport> = acquired
        .as_ref()
        .map(|source| {
            apply_mcp_host_install(
                input.integration_id.trim(),
                clean_optional(input.mode.as_deref()).unwrap_or(if cfg!(target_os = "windows") {
                    "local"
                } else {
                    "remote"
                }),
                input.host_path.as_deref(),
                &source.managed_root,
            )
        })
        .transpose()?;
    let installed_skills = if automatic && prepared.id == "ida-pro-mcp" {
        install_managed_repository_skills(
            tool,
            config_dir.clone(),
            &managed_integration_dir("ida-pro-mcp")?,
        )?
    } else {
        Vec::new()
    };
    let imported_skills = installed_skills.len();
    let mut manifest = McpInstallationManifest::new(tool, prepared.id, automatic);
    manifest.host_paths = prepared.host_paths.clone();
    if manifest.host_paths.is_empty() {
        if let Some(host) = clean_optional(input.host_path.as_deref())
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.exists())
        {
            manifest.host_paths.push(host.display().to_string());
        }
    }
    manifest.managed_paths = prepared.managed_paths.clone();
    manifest.installed_files = prepared.installed_files.clone();
    manifest.installed_skills = installed_skills;
    manifest.notes = prepared.notes.clone();
    if tool == ResourceToolId::Pi {
        manifest
            .notes
            .push("Pi MCP Adapter 作为共享扩展安装，卸载单个 MCP 时会保留".to_string());
    }
    let state: SkillsMcpState = match install_tool_mcp_config_inner(
        tool,
        config_dir.clone(),
        prepared.id,
        prepared.name,
        prepared.config,
    ) {
        Ok(state) => state,
        Err(error) => {
            if let Some(report) = host_report.as_ref() {
                rollback_mcp_host_install(report);
            }
            return Err(error);
        }
    };
    save_installation_manifest(&manifest)?;
    Ok(SkillsMcpActionResult {
        imported_skills,
        imported_mcp: 1,
        message: if let Some(source) = acquired {
            let mut message = format!(
                "已获取并校验 {}，为 {} 配置 {}；托管文件位于 {}",
                source.version,
                tool.label(),
                prepared.name,
                source.managed_root.display()
            );
            if let Some(report) = host_report.as_ref() {
                message.push_str("；");
                message.push_str(&report.plan.message);
                if report.installed > 0 {
                    message.push_str(&format!("（{} 个宿主文件）", report.installed));
                }
                if let Some(backup) = report.backup_location.as_ref() {
                    message.push_str(&format!("；宿主文件备份位于 {}", backup.display()));
                }
            }
            if let Some(next_step) = host_report
                .as_ref()
                .and_then(|report| report.plan.next_step.clone())
                .or(source.next_step)
            {
                message.push_str("；");
                message.push_str(&next_step);
            }
            message
        } else {
            format!(
                "已为 {} {} {}",
                tool.label(),
                if automatic {
                    "自动安装并配置"
                } else {
                    "手动配置"
                },
                prepared.name
            )
        },
        state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_dir(name: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "everything-patch-mcp-catalog-{name}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("create temp directory");
        path
    }

    fn input(id: &str) -> McpIntegrationInstallInput {
        McpIntegrationInstallInput {
            integration_id: id.to_string(),
            source_path: None,
            host_path: None,
            command: None,
            endpoint: None,
            mode: None,
            auto_install: None,
            source_mode: None,
        }
    }

    #[test]
    fn ida_uses_selected_project_without_running_installer() {
        let root = temp_dir("ida");
        fs::write(
            root.join("pyproject.toml"),
            "[project]\nname = \"ida-pro-mcp\"\nversion = \"2.0.0\"\n",
        )
        .expect("write pyproject");
        let mut request = input("ida-pro-mcp");
        request.source_path = Some(root.display().to_string());

        let prepared = prepare_integration(ResourceToolId::Codex, &request).expect("prepare IDA");

        assert_eq!(prepared.id, "ida-pro-mcp");
        assert_eq!(prepared.config["command"], "uv");
        assert_eq!(prepared.config["args"][0], "run");
        assert_eq!(prepared.config["args"][1], "--offline");
        assert_eq!(prepared.config["args"][2], "--no-sync");
        assert_eq!(prepared.config["args"][3], "--project");
        assert_eq!(prepared.config["args"][6], "--stdio");
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn cheat_engine_remote_mode_sets_tcp_environment() {
        let root = temp_dir("ce");
        let server = root.join("MCP_Server");
        fs::create_dir_all(&server).expect("create server directory");
        fs::write(server.join("mcp_cheatengine.py"), "# bridge\n").expect("write bridge");
        fs::write(server.join("ce_mcp_bridge.lua"), "-- bridge\n").expect("write lua");
        let mut request = input("cheatengine-mcp");
        request.source_path = Some(root.display().to_string());
        request.mode = Some("remote".to_string());
        request.endpoint = Some("192.0.2.10:4567".to_string());

        let prepared = prepare_integration(ResourceToolId::Claude, &request).expect("prepare CE");

        assert_eq!(prepared.config["env"]["CE_MCP_TRANSPORT"], "tcp");
        assert_eq!(prepared.config["env"]["CE_MCP_HOST"], "192.0.2.10");
        assert_eq!(prepared.config["env"]["CE_MCP_PORT"], "4567");
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn x64dbg_uses_python_bridge_and_normalized_endpoint() {
        let root = temp_dir("x64dbg");
        let script = root.join("x64dbg.py");
        fs::write(&script, "# bridge\n").expect("write bridge");
        let mut request = input("x64dbg-mcp");
        request.source_path = Some(script.display().to_string());
        request.mode = Some("remote".to_string());
        request.endpoint = Some("http://debug-host.example:8888".to_string());

        let prepared = prepare_integration(ResourceToolId::Grok, &request).expect("prepare x64dbg");

        assert_eq!(
            prepared.config["env"]["X64DBG_URL"],
            "http://debug-host.example:8888/"
        );
        assert_eq!(prepared.config["args"][0], script.display().to_string());
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn burp_direct_transport_is_used_for_sse_capable_targets() {
        let request = input("burp-suite-mcp");

        let claude =
            prepare_integration(ResourceToolId::Claude, &request).expect("prepare Claude Burp");
        let zcode =
            prepare_integration(ResourceToolId::Zcode, &request).expect("prepare ZCode Burp");
        let kilo =
            prepare_integration(ResourceToolId::Kilo, &request).expect("prepare Kilo Burp");
        let pi = prepare_integration(ResourceToolId::Pi, &request).expect("prepare Pi Burp");

        assert_eq!(claude.config["url"], "http://127.0.0.1:9876/sse");
        assert_eq!(claude.config["type"], "sse");
        assert_eq!(zcode.config, claude.config);
        assert_eq!(kilo.config, claude.config);
        assert_eq!(pi.config, claude.config);
    }

    #[test]
    fn burp_direct_transport_rejects_targets_without_legacy_sse_support() {
        let mut request = input("burp-suite-mcp");
        request.mode = Some("direct".to_string());

        for tool in [ResourceToolId::Codex, ResourceToolId::Grok] {
            let error = prepare_integration(tool, &request).expect_err("reject direct SSE");
            assert!(error.to_string().contains("mcp-proxy-all.jar"));
        }
    }

    #[test]
    fn burp_proxy_uses_selected_jar_without_running_it() {
        let root = temp_dir("burp-proxy");
        let proxy = root.join("mcp-proxy-all.jar");
        fs::write(&proxy, "test fixture").expect("write proxy fixture");
        let mut request = input("burp-suite-mcp");
        request.mode = Some("proxy".to_string());
        request.source_path = Some(proxy.display().to_string());
        request.command = Some("java".to_string());
        request.endpoint = Some("http://127.0.0.1:9876/sse".to_string());

        let prepared =
            prepare_integration(ResourceToolId::Zcode, &request).expect("prepare Burp proxy");

        assert_eq!(prepared.config["command"], "java");
        assert_eq!(prepared.config["args"][0], "-jar");
        assert_eq!(prepared.config["args"][1], proxy.display().to_string());
        assert_eq!(prepared.config["args"][2], "--sse-url");
        assert_eq!(prepared.config["args"][3], "http://127.0.0.1:9876/sse");
        fs::remove_dir_all(root).expect("remove temp directory");
    }

    #[test]
    fn source_paths_must_be_absolute() {
        let mut request = input("x64dbg-mcp");
        request.source_path = Some("src/x64dbg.py".to_string());
        request.mode = Some("remote".to_string());

        let error =
            prepare_integration(ResourceToolId::Codex, &request).expect_err("reject relative path");

        assert!(error.to_string().contains("绝对路径"));
    }

    #[test]
    fn old_clients_keep_manual_source_behavior() {
        let request = input("ida-pro-mcp");
        assert_eq!(
            source_mode(&request).expect("default source mode"),
            "manual"
        );
    }

    #[test]
    fn source_mode_rejects_unknown_values() {
        let mut request = input("ida-pro-mcp");
        request.source_mode = Some("automatic".to_string());

        let error = source_mode(&request).expect_err("reject unknown source mode");

        assert!(error.to_string().contains("文件来源模式"));
    }

    #[test]
    fn managed_burp_transport_matches_target_capabilities() {
        let request = input("burp-suite-mcp");

        let (claude_mode, _) =
            managed_acquisition_settings(ToolId::Claude, &request).expect("Claude settings");
        let (codex_mode, _) =
            managed_acquisition_settings(ToolId::Codex, &request).expect("Codex settings");

        assert_eq!(claude_mode, "direct");
        assert_eq!(codex_mode, "proxy");
    }
}
