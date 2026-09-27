//! 从 DevConduit 官方库 `~/.everything-patch/everything-patch.db` 一键导入提示词。
//!
//! 目标用户：已在 DevConduit 里积累大量破甲提示词、切换到 Cockpit 独立库后想
//! 一次性带过来。只读旧库，不修改它；写入 Cockpit 新库 `~/.cockpit/devconduit.db`。

use crate::devconduit::db::open_db;
use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::paths::home_dir;
use rusqlite::{params, Connection};
use std::path::PathBuf;

/// 旧的 DevConduit 库可能位于 `~/.everything-patch/everything-patch.db`。
fn legacy_database_candidates() -> Result<Vec<PathBuf>> {
    let home = home_dir()?;
    Ok(vec![
        home.join(".everything-patch").join("everything-patch.db"),
        home.join(".codexx").join("everything-patch.db"),
    ])
}

/// 找出第一个可读的旧库。
fn find_legacy_db() -> Option<PathBuf> {
    legacy_database_candidates()
        .ok()?
        .into_iter()
        .find(|path| path.is_file())
}

fn count_saved_prompts(conn: &Connection, engine: &str) -> Result<u64> {
    conn.query_row(
        "SELECT COUNT(*) FROM prompts WHERE engine = ?1",
        params![engine],
        |row| row.get(0),
    )
    .map_err(|e| CodexxError::Database(e.to_string()))
}

/// 一次性导入（幂等：按 id 冲突则跳过）。
///
/// 返回每种引擎导入数量，以及被跳过的数量。
pub(crate) fn import_prompts_from_devconduit(
    engine_filter: Option<String>,
    dry_run: bool,
) -> Result<(u64, u64)> {
    let Some(legacy) = find_legacy_db() else {
        return Err(CodexxError::Config(format!(
            "未找到 DevConduit 旧库（{}）",
            legacy_database_candidates()?
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" 或 ")
        )));
    };
    let source = Connection::open(&legacy).map_err(|e| CodexxError::Database(e.to_string()))?;

    // 兼容旧库没有 rules_content 列
    let mut stmt = source
        .prepare("PRAGMA table_info(prompts)")
        .map_err(|e| CodexxError::Database(e.to_string()))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| CodexxError::Database(e.to_string()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|e| CodexxError::Database(e.to_string()))?;
    let has_rules = columns.iter().any(|c| c == "rules_content");

    let mut rows = source
        .prepare(&format!(
            "SELECT id, title, filename, content, engine, created_at, updated_at{} FROM prompts ORDER BY updated_at DESC",
            if has_rules { ", rules_content" } else { "" }
        ))
        .map_err(|e| CodexxError::Database(e.to_string()))?;

    let existing = open_db()?;
    let filter = engine_filter.as_deref();
    let mut imported = 0u64;
    let mut skipped = 0u64;

    let row_iter = rows
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                if has_rules {
                    row.get::<_, Option<String>>(7)?
                } else {
                    None
                },
            ))
        })
        .map_err(|e| CodexxError::Database(e.to_string()))?;

    if dry_run {
        for item in row_iter {
            let item = item.map_err(|e| CodexxError::Database(e.to_string()))?;
            if filter.is_some_and(|f| !item.4.eq_ignore_ascii_case(f)) {
                continue;
            }
            if existing
                .query_row(
                    "SELECT 1 FROM prompts WHERE id = ?1",
                    params![item.0],
                    |_| Ok(()),
                )
                .is_ok()
            {
                skipped += 1;
            } else {
                imported += 1;
            }
        }
        return Ok((imported, skipped));
    }

    for item in row_iter {
        let item = item.map_err(|e| CodexxError::Database(e.to_string()))?;
        if filter.is_some_and(|f| !item.4.eq_ignore_ascii_case(f)) {
            continue;
        }
        let already = existing
            .query_row("SELECT 1 FROM prompts WHERE id = ?1", params![item.0], |_| Ok(()))
            .is_ok();
        if already {
            skipped += 1;
            continue;
        }
        existing
            .execute(
                "INSERT OR IGNORE INTO prompts
                    (id, title, filename, content, engine, created_at, updated_at, rules_content)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    item.0,
                    item.1,
                    item.2,
                    item.3,
                    item.4,
                    item.5,
                    item.6,
                    item.7,
                ],
            )
            .map_err(|e| CodexxError::Database(e.to_string()))?;
        imported += 1;
    }

    Ok((imported, skipped))
}

#[tauri::command]
pub(crate) async fn dc_import_devconduit_prompts(
    engine_filter: Option<String>,
    dry_run: Option<bool>,
) -> Result<u64> {
    tauri::async_runtime::spawn_blocking(move || {
        let (imported, skipped) = import_prompts_from_devconduit(engine_filter, dry_run.unwrap_or(false))?;
        if dry_run.unwrap_or(false) {
            // 预演：返回预演标记（前端用单独状态展示）
            let _ = skipped;
        }
        Ok(imported)
    })
    .await
    .map_err(|e| CodexxError::Config(format!("导入 DevConduit 提示词失败: {e}")))?
}
