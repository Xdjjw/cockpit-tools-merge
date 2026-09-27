use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpHostCandidate {
    pub(crate) path: String,
    pub(crate) executable_path: String,
    pub(crate) source: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpHostDiscovery {
    pub(crate) integration_id: String,
    pub(crate) candidates: Vec<McpHostCandidate>,
}

#[derive(Clone, Copy)]
enum HostKind {
    Ida,
    CheatEngine,
    X64dbg,
}

impl HostKind {
    fn integration_id(self) -> &'static str {
        match self {
            Self::Ida => "ida-pro-mcp",
            Self::CheatEngine => "cheatengine-mcp",
            Self::X64dbg => "x64dbg-mcp",
        }
    }
}

fn path_key(path: &Path) -> String {
    let value = path
        .to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches(['\\', '/'])
        .to_string();
    if cfg!(target_os = "windows") {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

fn first_file(root: &Path, relatives: &[&str]) -> Option<PathBuf> {
    relatives
        .iter()
        .map(|relative| root.join(relative))
        .find(|path| path.is_file())
}

fn normalize_root(kind: HostKind, selected: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut roots = Vec::new();
    if selected.is_file() {
        if let Some(parent) = selected.parent() {
            roots.push(parent.to_path_buf());
            if let Some(grandparent) = parent.parent() {
                roots.push(grandparent.to_path_buf());
                if let Some(great_grandparent) = grandparent.parent() {
                    roots.push(great_grandparent.to_path_buf());
                }
            }
        }
    } else {
        roots.push(selected.to_path_buf());
    }

    for root in roots {
        let executable = match kind {
            HostKind::Ida => first_file(&root, &["ida.exe", "ida64.exe"]),
            HostKind::CheatEngine => first_file(
                &root,
                &[
                    "Cheat Engine.exe",
                    "cheatengine-x86_64.exe",
                    "cheatengine-i386.exe",
                ],
            ),
            HostKind::X64dbg => first_file(
                &root,
                &[
                    "release/x64/x64dbg.exe",
                    "x64/x64dbg.exe",
                    "x64dbg.exe",
                    "release/x32/x32dbg.exe",
                    "x32/x32dbg.exe",
                    "x32dbg.exe",
                ],
            ),
        }?;
        let valid = match kind {
            HostKind::Ida => root.join("idalib/python/py-activate-idalib.py").is_file(),
            HostKind::CheatEngine | HostKind::X64dbg => true,
        };
        if valid {
            let normalized_root = if matches!(kind, HostKind::X64dbg)
                && root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.eq_ignore_ascii_case("x64") || name.eq_ignore_ascii_case("x32")
                    })
                && root
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.eq_ignore_ascii_case("release"))
            {
                root.parent()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf)
                    .unwrap_or(root)
            } else {
                root
            };
            return Some((normalized_root, executable));
        }
    }
    None
}

fn push_candidate(
    kind: HostKind,
    raw: PathBuf,
    source: &str,
    output: &mut Vec<McpHostCandidate>,
    seen: &mut HashSet<String>,
) {
    let Some((root, executable)) = normalize_root(kind, &raw) else {
        return;
    };
    let key = path_key(&root);
    if !seen.insert(key) {
        return;
    }
    output.push(McpHostCandidate {
        path: root.display().to_string(),
        executable_path: executable.display().to_string(),
        source: source.to_string(),
    });
}

fn environment_candidates(kind: HostKind) -> Vec<PathBuf> {
    let names: &[&str] = match kind {
        HostKind::Ida => &["IDA_PATH", "IDADIR"],
        HostKind::CheatEngine => &["CHEAT_ENGINE_PATH", "CE_PATH"],
        HostKind::X64dbg => &["X64DBG_PATH"],
    };
    names
        .iter()
        .filter_map(|name| std::env::var_os(name))
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .collect()
}

#[cfg(target_os = "windows")]
fn registry_candidates(kind: HostKind) -> Vec<PathBuf> {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    fn display_matches(kind: HostKind, display_name: &str) -> bool {
        let name = display_name.to_ascii_lowercase();
        match kind {
            HostKind::Ida => {
                name.contains("ida") && (name.contains("professional") || name.contains("pro"))
            }
            HostKind::CheatEngine => name.starts_with("cheat engine"),
            HostKind::X64dbg => name.contains("x64dbg"),
        }
    }

    fn display_icon_path(value: &str) -> Option<PathBuf> {
        let trimmed = value.trim().trim_matches('"');
        let without_index = trimmed
            .rsplit_once(',')
            .filter(|(_, index)| index.trim().parse::<i32>().is_ok())
            .map(|(path, _)| path)
            .unwrap_or(trimmed)
            .trim_matches('"');
        (!without_index.is_empty()).then(|| PathBuf::from(without_index))
    }

    let roots = [
        (HKEY_CURRENT_USER, KEY_READ),
        (HKEY_CURRENT_USER, KEY_READ | KEY_WOW64_64KEY),
        (HKEY_CURRENT_USER, KEY_READ | KEY_WOW64_32KEY),
        (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY),
        (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY),
    ];
    let mut result = Vec::new();
    for (hive, flags) in roots {
        let Ok(uninstall) = RegKey::predef(hive).open_subkey_with_flags(
            r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
            flags,
        ) else {
            continue;
        };
        for name in uninstall.enum_keys().flatten() {
            let Ok(entry) = uninstall.open_subkey_with_flags(&name, flags) else {
                continue;
            };
            let display_name = entry
                .get_value::<String, _>("DisplayName")
                .unwrap_or_default();
            if !display_matches(kind, &display_name) {
                continue;
            }
            if let Ok(location) = entry.get_value::<String, _>("InstallLocation") {
                let location = location.trim().trim_matches('"');
                if !location.is_empty() {
                    result.push(PathBuf::from(location));
                }
            }
            if let Ok(icon) = entry.get_value::<String, _>("DisplayIcon") {
                if let Some(path) = display_icon_path(&icon) {
                    result.push(path);
                }
            }
        }
    }
    result
}

#[cfg(not(target_os = "windows"))]
fn registry_candidates(_kind: HostKind) -> Vec<PathBuf> {
    Vec::new()
}

fn common_candidates(kind: HostKind) -> Vec<PathBuf> {
    let mut result = Vec::new();
    #[cfg(target_os = "windows")]
    {
        let mut roots = Vec::new();
        // Portable installations are often placed on D:/ or another volume
        // and do not create an uninstall registry entry.
        for letter in b'A'..=b'Z' {
            let root = PathBuf::from(format!("{}:\\", letter as char));
            if root.is_dir() {
                roots.push(root);
            }
        }
        for name in ["ProgramFiles", "ProgramFiles(x86)"] {
            if let Some(path) = std::env::var_os(name).map(PathBuf::from) {
                roots.push(path);
            }
        }
        match kind {
            HostKind::Ida => {
                for root in roots {
                    let locations = [
                        root.clone(),
                        root.join("Program Files"),
                        root.join("Program Files (x86)"),
                    ];
                    for location in locations {
                        if let Ok(entries) = fs::read_dir(&location) {
                            result.extend(entries.flatten().map(|entry| entry.path()).filter(
                                |path| {
                                    path.file_name().and_then(|name| name.to_str()).is_some_and(
                                        |name| {
                                            let name = name.to_ascii_lowercase();
                                            name.contains("ida")
                                                && (name.contains("professional")
                                                    || name.contains("pro"))
                                        },
                                    )
                                },
                            ));
                        }
                    }
                }
            }
            HostKind::CheatEngine => {
                result.extend(roots.iter().flat_map(|root| {
                    [
                        root.join("Cheat Engine"),
                        root.join("Program Files/Cheat Engine"),
                        root.join("Program Files (x86)/Cheat Engine"),
                    ]
                }));
            }
            HostKind::X64dbg => {
                result.extend(roots.iter().flat_map(|root| {
                    [
                        root.join("x64dbg"),
                        root.join("Program Files/x64dbg"),
                        root.join("Program Files (x86)/x64dbg"),
                    ]
                }));
                if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
                    result.push(local.join("Programs/x64dbg"));
                }
            }
        }
    }
    #[cfg(target_os = "macos")]
    if matches!(kind, HostKind::Ida) {
        let applications = Path::new("/Applications");
        if let Ok(entries) = fs::read_dir(applications) {
            result.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        name.starts_with("IDA Professional") && name.ends_with(".app")
                    })
            }));
        }
    }
    result
}

fn discover_kind(kind: HostKind) -> McpHostDiscovery {
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for path in environment_candidates(kind) {
        push_candidate(kind, path, "environment", &mut candidates, &mut seen);
    }
    for path in registry_candidates(kind) {
        push_candidate(kind, path, "registry", &mut candidates, &mut seen);
    }
    for path in common_candidates(kind) {
        push_candidate(kind, path, "common", &mut candidates, &mut seen);
    }
    McpHostDiscovery {
        integration_id: kind.integration_id().to_string(),
        candidates,
    }
}

pub(crate) fn discover_mcp_hosts_inner() -> Vec<McpHostDiscovery> {
    [HostKind::Ida, HostKind::CheatEngine, HostKind::X64dbg]
        .into_iter()
        .map(discover_kind)
        .collect()
}

pub(crate) fn validate_host_path(integration_id: &str, path: &Path) -> Option<(PathBuf, PathBuf)> {
    let kind = match integration_id {
        "ida-pro-mcp" => HostKind::Ida,
        "cheatengine-mcp" => HostKind::CheatEngine,
        "x64dbg-mcp" => HostKind::X64dbg,
        _ => return None,
    };
    normalize_root(kind, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    fn temp_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "everything-patch-host-discovery-{label}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create fixture");
        root
    }

    #[test]
    fn validates_ida_root_independent_of_drive_letter() {
        let root = temp_dir("ida");
        fs::write(root.join("ida.exe"), "fixture").expect("write ida");
        let activation = root.join("idalib/python/py-activate-idalib.py");
        fs::create_dir_all(activation.parent().expect("activation parent")).expect("create idalib");
        fs::write(&activation, "# fixture").expect("write activation");

        let normalized = validate_host_path("ida-pro-mcp", &root).expect("valid IDA");
        assert_eq!(normalized.0, root);
        assert!(normalized.1.ends_with("ida.exe"));
        fs::remove_dir_all(normalized.0).expect("remove fixture");
    }

    #[test]
    fn normalizes_portable_x64dbg_distribution_root() {
        let root = temp_dir("x64dbg");
        let executable = root.join("release/x64/x64dbg.exe");
        fs::create_dir_all(executable.parent().expect("x64 parent")).expect("create x64");
        fs::write(&executable, "fixture").expect("write x64dbg");

        let normalized = validate_host_path("x64dbg-mcp", &executable).expect("valid x64dbg");
        assert_eq!(normalized.0, root);
        assert_eq!(normalized.1, executable);
        fs::remove_dir_all(normalized.0).expect("remove fixture");
    }
}
