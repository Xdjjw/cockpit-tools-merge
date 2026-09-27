use super::skills::sanitize_dir_name;
use super::types::ResourceToolId;
use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::file_io::io_err;
use crate::devconduit::paths::app_home;
use crate::devconduit::{now_rfc3339, open_db};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct McpInstallationManifest {
    pub(crate) version: u32,
    pub(crate) integration_id: String,
    pub(crate) app_type: String,
    pub(crate) automatic: bool,
    #[serde(default)]
    pub(crate) host_paths: Vec<String>,
    #[serde(default)]
    pub(crate) managed_paths: Vec<String>,
    #[serde(default)]
    pub(crate) installed_files: Vec<String>,
    #[serde(default)]
    pub(crate) installed_skills: Vec<String>,
    #[serde(default)]
    pub(crate) notes: Vec<String>,
}

impl McpInstallationManifest {
    pub(crate) fn new(tool: ResourceToolId, integration_id: &str, automatic: bool) -> Self {
        Self {
            version: 1,
            integration_id: integration_id.to_string(),
            app_type: tool.as_str().to_string(),
            automatic,
            ..Self::default()
        }
    }
}

fn manifest_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<McpInstallationManifest> {
    let text = row.get::<_, String>(0)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

pub(crate) fn load_installation_manifest(
    tool: ResourceToolId,
    integration_id: &str,
) -> Result<Option<McpInstallationManifest>> {
    let connection = open_db()?;
    let mut statement = connection
        .prepare(
            "SELECT manifest_json FROM managed_mcp_installations
             WHERE app_type = ?1 AND resource_id = ?2",
        )
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    match statement.query_row(params![tool.as_str(), integration_id], manifest_from_row) {
        Ok(manifest) => Ok(Some(manifest)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(CodexxError::Database(error.to_string())),
    }
}

fn merge_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !value.is_empty() && !target.iter().any(|existing| existing == value) {
            target.push(value.clone());
        }
    }
}

pub(crate) fn save_installation_manifest(manifest: &McpInstallationManifest) -> Result<()> {
    let mut merged = load_installation_manifest(
        ResourceToolId::parse(&manifest.app_type)?,
        &manifest.integration_id,
    )?
    .unwrap_or_else(|| manifest.clone());
    if merged.integration_id.is_empty() {
        merged = manifest.clone();
    } else {
        merged.version = manifest.version.max(merged.version);
        merged.automatic |= manifest.automatic;
        merge_unique(&mut merged.host_paths, &manifest.host_paths);
        merge_unique(&mut merged.managed_paths, &manifest.managed_paths);
        merge_unique(&mut merged.installed_files, &manifest.installed_files);
        merge_unique(&mut merged.installed_skills, &manifest.installed_skills);
        merge_unique(&mut merged.notes, &manifest.notes);
    }
    let connection = open_db()?;
    connection
        .execute(
            "INSERT INTO managed_mcp_installations
               (app_type, resource_id, manifest_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(app_type, resource_id) DO UPDATE SET
               manifest_json = excluded.manifest_json,
               updated_at = excluded.updated_at",
            params![
                merged.app_type,
                merged.integration_id,
                serde_json::to_string(&merged).map_err(|error| CodexxError::Config(format!(
                    "序列化 MCP 安装清单失败: {error}"
                )))?,
                now_rfc3339(),
            ],
        )
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    Ok(())
}

pub(crate) fn delete_installation_manifest(
    tool: ResourceToolId,
    integration_id: &str,
) -> Result<()> {
    let connection = open_db()?;
    connection
        .execute(
            "DELETE FROM managed_mcp_installations
             WHERE app_type = ?1 AND resource_id = ?2",
            params![tool.as_str(), integration_id],
        )
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    Ok(())
}

pub(crate) fn forget_installed_skill(tool: ResourceToolId, directory: &str) -> Result<()> {
    let directory = sanitize_dir_name(directory, "skill");
    let connection = open_db()?;
    let mut statement = connection
        .prepare(
            "SELECT app_type, resource_id, manifest_json
             FROM managed_mcp_installations",
        )
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    let mut updates = Vec::new();
    for row in rows {
        let (app_type, resource_id, text) =
            row.map_err(|error| CodexxError::Database(error.to_string()))?;
        if app_type != tool.as_str() {
            continue;
        }
        let mut manifest: McpInstallationManifest = serde_json::from_str(&text).unwrap_or_default();
        let before = manifest.installed_skills.len();
        manifest
            .installed_skills
            .retain(|value| sanitize_dir_name(value, "skill") != directory);
        if manifest.installed_skills.len() != before {
            updates.push((app_type, resource_id, manifest));
        }
    }
    drop(statement);
    for (app_type, resource_id, manifest) in updates {
        let text = serde_json::to_string(&manifest)
            .map_err(|error| CodexxError::Config(format!("序列化 MCP 安装清单失败: {error}")))?;
        connection
            .execute(
                "UPDATE managed_mcp_installations
                 SET manifest_json = ?1, updated_at = ?2
                 WHERE app_type = ?3 AND resource_id = ?4",
                params![text, now_rfc3339(), app_type, resource_id],
            )
            .map_err(|error| CodexxError::Database(error.to_string()))?;
    }
    Ok(())
}

fn other_manifests(
    tool: ResourceToolId,
    integration_id: &str,
) -> Result<Vec<McpInstallationManifest>> {
    let connection = open_db()?;
    let mut statement = connection
        .prepare(
            "SELECT manifest_json FROM managed_mcp_installations
             WHERE resource_id = ?1 AND app_type <> ?2",
        )
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    let rows = statement
        .query_map(params![integration_id, tool.as_str()], |row| {
            manifest_from_row(row)
        })
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    let mut manifests = Vec::new();
    for row in rows {
        let manifest = row.map_err(|error| CodexxError::Database(error.to_string()))?;
        if manifest.integration_id == integration_id {
            manifests.push(manifest);
        }
    }
    Ok(manifests)
}

fn other_resource_targets(
    tool: ResourceToolId,
    integration_id: &str,
) -> Result<Vec<ResourceToolId>> {
    let connection = open_db()?;
    let mut statement = connection
        .prepare(
            "SELECT app_type FROM managed_mcp_targets
             WHERE resource_id = ?1 AND app_type <> ?2",
        )
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    let rows = statement
        .query_map(params![integration_id, tool.as_str()], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|error| CodexxError::Database(error.to_string()))?;
    let mut targets = Vec::new();
    for row in rows {
        let app_type = row.map_err(|error| CodexxError::Database(error.to_string()))?;
        if let Ok(target) = ResourceToolId::parse(&app_type) {
            targets.push(target);
        }
    }
    Ok(targets)
}

fn path_key(path: &str) -> String {
    let path = PathBuf::from(path);
    let normalized = fs::canonicalize(&path).unwrap_or(path);
    let value = normalized.to_string_lossy().replace('/', "\\");
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

fn referenced_paths(manifests: &[McpInstallationManifest]) -> HashSet<String> {
    manifests
        .iter()
        .flat_map(|manifest| {
            manifest
                .managed_paths
                .iter()
                .chain(manifest.installed_files.iter())
        })
        .map(|path| path_key(path))
        .collect()
}

fn is_referenced(path: &str, referenced: &HashSet<String>) -> bool {
    let key = path_key(path);
    referenced.iter().any(|other| {
        key == *other
            || key
                .strip_prefix(other)
                .is_some_and(|suffix| suffix.starts_with('\\'))
            || other
                .strip_prefix(&key)
                .is_some_and(|suffix| suffix.starts_with('\\'))
    })
}

fn remove_file_if_unreferenced(path: &str, referenced: &HashSet<String>) -> Result<()> {
    if is_referenced(path, referenced) {
        return Ok(());
    }
    let path = Path::new(path);
    if !path.exists() {
        return Ok(());
    }
    if !path.is_file() {
        return Err(CodexxError::Config(format!(
            "安装清单中的目标不是文件: {}",
            path.display()
        )));
    }
    fs::remove_file(path).map_err(|error| io_err(path, error))
}

fn remove_directory_if_unreferenced(path: &str, referenced: &HashSet<String>) -> Result<()> {
    if is_referenced(path, referenced) {
        return Ok(());
    }
    let path = Path::new(path);
    if !path.exists() {
        return Ok(());
    }
    if !path.is_dir() {
        return Err(CodexxError::Config(format!(
            "安装清单中的目标不是目录: {}",
            path.display()
        )));
    }
    fs::remove_dir_all(path).map_err(|error| io_err(path, error))
}

fn remove_skill_copy(
    tool: ResourceToolId,
    config_dir: Option<String>,
    directory: &str,
) -> Result<()> {
    let directory = sanitize_dir_name(directory, "skill");
    let active = tool.skills_dir(config_dir)?;
    let disabled = app_home()?.join("disabled-skills").join(tool.as_str());
    for base in [active, disabled] {
        let path = base.join(&directory);
        if path.exists() {
            fs::remove_dir_all(&path).map_err(|error| io_err(&path, error))?;
        }
    }
    Ok(())
}

/// Remove files created by an automatic catalog install. Shared repositories
/// and plugins are retained while another resource target still references
/// the same path.
pub(crate) fn remove_installation_artifacts(
    tool: ResourceToolId,
    config_dir: Option<String>,
    integration_id: &str,
) -> Result<Vec<String>> {
    let Some(manifest) = load_installation_manifest(tool, integration_id)? else {
        return Ok(Vec::new());
    };
    let other_targets = other_resource_targets(tool, integration_id)?;
    for target in &other_targets {
        let mut inherited = manifest.clone();
        inherited.app_type = target.as_str().to_string();
        inherited.installed_skills.clear();
        save_installation_manifest(&inherited)?;
    }
    let others = other_manifests(tool, integration_id)?;
    let mut referenced = referenced_paths(&others);
    if !other_targets.is_empty() {
        referenced.extend(
            manifest
                .managed_paths
                .iter()
                .chain(manifest.installed_files.iter())
                .map(|path| path_key(path)),
        );
    }
    for directory in &manifest.installed_skills {
        remove_skill_copy(tool, config_dir.clone(), directory)?;
    }
    for file in &manifest.installed_files {
        remove_file_if_unreferenced(file, &referenced)?;
    }
    for directory in &manifest.managed_paths {
        remove_directory_if_unreferenced(directory, &referenced)?;
    }
    delete_installation_manifest(tool, integration_id)?;
    let mut notes = manifest.notes;
    if !manifest.host_paths.is_empty() {
        notes.push(format!(
            "已清理 {} 的 DevConduit 管理文件；宿主目录保留: {}",
            integration_id,
            manifest.host_paths.join("; ")
        ));
    }
    Ok(notes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_merges_unique_paths() {
        let mut values = vec!["a".to_string()];
        merge_unique(&mut values, &["a".to_string(), "b".to_string()]);
        assert_eq!(values, vec!["a", "b"]);
    }

    #[test]
    fn nested_paths_are_protected_by_a_shared_parent() {
        let referenced = HashSet::from([path_key("C:\\shared\\integration")]);
        assert!(is_referenced("C:\\shared\\integration\\.venv", &referenced));
        assert!(!is_referenced("C:\\other\\integration", &referenced));
    }
}
