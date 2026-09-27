//! Cockpit 版 DevConduit 独立 SQLite 库。
//!
//! 数据存 `~/.cockpit/devconduit.db`（`DEV_CONDUIT_HOME` 可改目录）。
//! 与 DevConduit 官方 `~/.everything-patch/everything-patch.db` 相互独立；
//! 需要把旧库的破甲提示词带过来时使用 `import_devconduit` 模块。

use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::file_io::ensure_directory;
use crate::devconduit::paths::app_home;
use rusqlite::Connection;
use std::path::PathBuf;

/// 数据目录下的库文件名。
pub(crate) const DB_FILENAME: &str = "devconduit.db";

pub(crate) fn database_path() -> Result<PathBuf> {
    Ok(app_home()?.join(DB_FILENAME))
}

fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS prompts (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            filename TEXT NOT NULL,
            content TEXT NOT NULL,
            engine TEXT NOT NULL DEFAULT 'codex',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            rules_content TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_prompts_updated_at ON prompts(updated_at DESC);
        CREATE TABLE IF NOT EXISTS builtin_prompt_cache (
            id TEXT PRIMARY KEY,
            filename TEXT NOT NULL,
            source_url TEXT NOT NULL,
            content TEXT NOT NULL,
            checked_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS managed_mcp_servers (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            server_config TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS managed_skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            directory TEXT NOT NULL,
            source_path TEXT,
            content_hash TEXT,
            enabled INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS managed_mcp_targets (
            app_type TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (app_type, resource_id),
            FOREIGN KEY (resource_id) REFERENCES managed_mcp_servers(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS managed_skill_targets (
            app_type TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (app_type, resource_id),
            FOREIGN KEY (resource_id) REFERENCES managed_skills(id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS managed_mcp_installations (
            app_type TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            manifest_json TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (app_type, resource_id)
        );
        ",
    )
    .map_err(|e| CodexxError::Database(e.to_string()))?;

    // 兼容旧库没有 rules_content 列的情况
    ensure_column(
        conn,
        "prompts",
        "rules_content",
        "ALTER TABLE prompts ADD COLUMN rules_content TEXT",
    )?;

    // 存量清理：早期 seed 的 v3 与 external-* 记忆条目不再属于自定义区
    // （v3 已升级为内置模板；启用动作不再产生 external 记忆）。
    conn.execute_batch(
        "DELETE FROM prompts WHERE id = 'ultimate-unrestricted-v3';
         DELETE FROM prompts WHERE engine = 'codex' AND id LIKE 'external-%';",
    )
    .map_err(|e| CodexxError::Database(e.to_string()))?;

    // 老条目回填 target 关系（对齐 DevConduit 行为）
    conn.execute(
        "INSERT OR IGNORE INTO managed_mcp_targets (app_type, resource_id, enabled, updated_at)
         SELECT 'codex', id, enabled, updated_at FROM managed_mcp_servers",
        [],
    )
    .map_err(|e| CodexxError::Database(e.to_string()))?;
    conn.execute(
        "INSERT OR IGNORE INTO managed_skill_targets (app_type, resource_id, enabled, updated_at)
         SELECT 'codex', id, enabled, updated_at FROM managed_skills",
        [],
    )
    .map_err(|e| CodexxError::Database(e.to_string()))?;

    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    alter_sql: &str,
) -> Result<()> {
    let mut stmt = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| CodexxError::Database(e.to_string()))?;
    let mut columns = Vec::new();
    {
        let rows = stmt
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| CodexxError::Database(e.to_string()))?;
        for row in rows {
            columns.push(row.map_err(|e| CodexxError::Database(e.to_string()))?);
        }
    }
    if !columns.iter().any(|c| c == column) {
        conn.execute_batch(alter_sql)
            .map_err(|e| CodexxError::Database(e.to_string()))?;
    }
    Ok(())
}

/// 打开 Cockpit 独立 DevConduit 库（自动建父目录与 schema）。
pub(crate) fn open_db() -> Result<Connection> {
    let path = database_path()?;
    if let Some(parent) = path.parent() {
        ensure_directory(parent)?;
    }
    let conn = Connection::open(&path).map_err(|e| CodexxError::Database(e.to_string()))?;
    conn.execute_batch("PRAGMA journal_mode=WAL;")
        .map_err(|e| CodexxError::Database(e.to_string()))?;
    ensure_schema(&conn)?;
    Ok(conn)
}


/// 现有数据库连接，用于需要多连接复用的场合。
pub(crate) fn connection_is_managed(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name == DB_FILENAME)
}
