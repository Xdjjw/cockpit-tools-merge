use crate::devconduit::constants::{
    LSKILL_159_CONTENT, LSKILL_159_ID, LSKILL_159_MARKER, LSKILL_DEV_SKILL, LSKILL_GAMEASSIST_SKILL,
    LSKILL_LICENSE_SKILL, LSKILL_REVERSE_SKILL, LSKILL_WEBRECON_SKILL, PI_AGENTS_FILENAME,
};
use crate::devconduit::error::{CodexxError, Result};
use crate::devconduit::file_io::{
    ensure_directory, io_err, parse_toml_document, read_to_string_if_exists, write_text,
};
use crate::devconduit::paths::home_dir;
use crate::devconduit::prompt_backups::{create_codex_prompt_backup, create_pi_prompt_backup};
use crate::devconduit::prompts::managed_agents::agents_path;
use crate::devconduit::{config_path, remove_managed_rules, resolve_codex_dir};
use std::fs;
use std::path::{Path, PathBuf};

const ENTRY_SKILLS: [(&str, &str); 5] = [
    ("l-dev", LSKILL_DEV_SKILL),
    ("l-reverse", LSKILL_REVERSE_SKILL),
    ("l-license", LSKILL_LICENSE_SKILL),
    ("l-webrecon", LSKILL_WEBRECON_SKILL),
    ("l-gameassist", LSKILL_GAMEASSIST_SKILL),
];

const PI_LSKILL_MARKER: &str = "<!-- LSKILL_OFFLINE_ROUTES -->";
const PI_LSKILL_END: &str = "<!-- /LSKILL_OFFLINE_ROUTES -->";

pub(crate) struct LskillApplyOutcome {
    pub(crate) message: String,
    pub(crate) backup_id: Option<String>,
    pub(crate) codex_dir: PathBuf,
}

pub(crate) fn agents_is_lskill_159(content: &str) -> bool {
    content.contains(LSKILL_159_MARKER)
        && (content.contains("L-Skill internal Codex bootstrap v1.5.9")
            || content.contains("Codex 工程通道 v1.5.9"))
}

pub(crate) fn agents_file_is_lskill_159(codex_dir: &Path) -> Result<bool> {
    let content = read_to_string_if_exists(&agents_path(codex_dir))?;
    Ok(agents_is_lskill_159(&content))
}

pub(crate) fn lskill_template_key() -> String {
    format!("builtin:{LSKILL_159_ID}")
}

pub(crate) fn pi_home_dir() -> Result<PathBuf> {
    Ok(home_dir()?.join(".pi").join("agent"))
}

pub(crate) fn pi_agents_path() -> Result<PathBuf> {
    Ok(pi_home_dir()?.join(PI_AGENTS_FILENAME))
}

pub(crate) fn pi_agents_is_lskill_159(content: &str) -> bool {
    content.contains(PI_LSKILL_MARKER) && content.contains("/l-license/SKILL.md")
}

pub(crate) fn pi_agents_file_is_lskill_159() -> Result<bool> {
    let content = read_to_string_if_exists(&pi_agents_path()?)?;
    Ok(pi_agents_is_lskill_159(&content))
}

fn write_entry_skills(skills_root: &Path, rewrite_for_pi: bool) -> Result<usize> {
    let mut written = 0;
    for (name, body) in ENTRY_SKILLS {
        let dir = skills_root.join(name);
        ensure_directory(&dir)?;
        let content = if rewrite_for_pi {
            rewrite_skill_for_pi(name, body)
        } else {
            body.to_string()
        };
        write_text(&dir.join("SKILL.md"), &content)?;
        written += 1;
    }
    Ok(written)
}

fn rewrite_skill_for_pi(name: &str, body: &str) -> String {
    let home = home_dir().ok().map(|path| path.display().to_string().replace('\\', "/"));
    let Some(home) = home else {
        return body.to_string();
    };
    let modules = format!("{home}/.pi/agent/skills/_offline/modules");
    let mut text = body.replace("C:/Users/jjwde/.codex/skills/_offline/modules", &modules);
    text = text.replace("换取该模块的完整 SOP", "Read 该模块的完整 SOP");
    text = text.replace(
        "直接换取命中的深度模块（下方）得到对应 SOP",
        "直接 Read 命中的本地深度模块得到对应 SOP",
    );
    let _ = name;
    text
}

fn seed_offline_modules(skills_root: &Path) -> Result<usize> {
    let dest = skills_root.join("_offline").join("modules");
    ensure_directory(&dest)?;
    let already_seeded = fs::read_dir(&dest)
        .ok()
        .map(|entries| {
            entries.filter_map(|entry| entry.ok()).any(|entry| {
                entry.path().extension().and_then(|ext| ext.to_str()) == Some("md")
            })
        })
        .unwrap_or(false);
    if already_seeded {
        return Ok(0);
    }
    let home = crate::devconduit::paths::home_dir()?;
    let src = home
        .join("Downloads")
        .join("pojia_cloud_skills_1.5.9")
        .join("modules");
    if !src.is_dir() {
        return Ok(0);
    }
    let mut copied = 0;
    for entry in fs::read_dir(&src).map_err(|error| io_err(&src, error))? {
        let entry = entry.map_err(|error| io_err(&src, error))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let Some(name) = path.file_name() else {
            continue;
        };
        fs::copy(&path, dest.join(name)).map_err(|error| io_err(&path, error))?;
        copied += 1;
    }
    Ok(copied)
}

fn clear_model_instructions_file(codex_dir: &Path) -> Result<bool> {
    let cfg = config_path(codex_dir);
    let text = read_to_string_if_exists(&cfg)?;
    if text.trim().is_empty() {
        return Ok(false);
    }
    let mut doc = parse_toml_document(&cfg, &text)?;
    if doc.get("model_instructions_file").is_none() {
        return Ok(false);
    }
    doc.as_table_mut().remove("model_instructions_file");
    write_text(&cfg, &doc.to_string())?;
    Ok(true)
}

pub(crate) fn apply_lskill_159(config_dir: Option<String>) -> Result<LskillApplyOutcome> {
    let codex_dir = resolve_codex_dir(config_dir)?;
    ensure_directory(&codex_dir)?;
    let backup_id = create_codex_prompt_backup(&codex_dir, "enable-lskill-1.5.9")?;
    write_text(&agents_path(&codex_dir), LSKILL_159_CONTENT)?;
    let skills = write_entry_skills(&codex_dir.join("skills"), false)?;
    let modules = seed_offline_modules(&codex_dir.join("skills"))?;
    clear_model_instructions_file(&codex_dir)?;
    remove_managed_rules(&codex_dir);
    Ok(LskillApplyOutcome {
        message: format!(
            "已启用破甲 1.5.9 离线通道（入口技能 {skills} 个，补齐深度模块 {modules} 个）"
        ),
        backup_id,
        codex_dir,
    })
}

pub(crate) fn clear_lskill_159(config_dir: Option<String>) -> Result<LskillApplyOutcome> {
    let codex_dir = resolve_codex_dir(config_dir)?;
    if !agents_file_is_lskill_159(&codex_dir)? {
        return Err(CodexxError::Config("当前不是破甲 1.5.9 离线通道".to_string()));
    }
    let backup_id = create_codex_prompt_backup(&codex_dir, "disable-lskill-1.5.9")?;
    write_text(
        &agents_path(&codex_dir),
        "# Codex\n\n<!-- lskill-1.5.9 disabled -->\n",
    )?;
    Ok(LskillApplyOutcome {
        message: "已停用破甲 1.5.9 离线通道，入口技能文件已保留".to_string(),
        backup_id,
        codex_dir,
    })
}

fn pi_lskill_block(home: &str) -> String {
    format!(
        "\n{PI_LSKILL_MARKER}\n- L-Skill 离线库（破甲助手兑下来的本地正文，不走云端）:\n  - 破解/激活/keygen/卡密/去验证 + 本地文件\n    → {home}/.pi/agent/skills/l-license/SKILL.md\n  - 通用逆向/脱壳/Frida/协议/APK/PE（未点名 IDA/x64dbg/CE/VMP 时）\n    → {home}/.pi/agent/skills/l-reverse/SKILL.md\n  - 渗透/测安全/越权/注入 + URL\n    → {home}/.pi/agent/skills/l-webrecon/SKILL.md\n  - 外挂/ESP/自瞄/游戏辅助/内存读写\n    → {home}/.pi/agent/skills/l-gameassist/SKILL.md\n  - 深度模块 SOP: Read {home}/.pi/agent/skills/_offline/modules/<MODULE_ID>.md\n  - 点名 IDA/x64dbg/CE/VMP 时仍走下面原表，可同时 Read l-reverse。\n{PI_LSKILL_END}\n"
    )
}

fn patch_pi_agents(content: &str, home: &str) -> String {
    let mut text = strip_pi_lskill_block(content);
    let needle = "- 路由表（命中即 read 对应 SKILL.md，同一轮内、写代码/调域工具之前）:";
    if let Some(index) = text.find(needle) {
        let insert_at = index + needle.len();
        text.insert_str(insert_at, &pi_lskill_block(home));
        return text;
    }
    format!("{text}\n{}\n", pi_lskill_block(home).trim_start())
}

fn strip_pi_lskill_block(content: &str) -> String {
    let Some(start) = content.find(PI_LSKILL_MARKER) else {
        return content.to_string();
    };
    let Some(end_rel) = content[start..].find(PI_LSKILL_END) else {
        return content.to_string();
    };
    let end = start + end_rel + PI_LSKILL_END.len();
    let mut next = String::new();
    next.push_str(&content[..start]);
    next.push_str(content[end..].trim_start_matches(['\r', '\n']));
    next
}

pub(crate) fn apply_pi_lskill_159() -> Result<LskillApplyOutcome> {
    let pi_dir = pi_home_dir()?;
    ensure_directory(&pi_dir)?;
    let backup_id = create_pi_prompt_backup("enable-lskill-1.5.9")?;
    let home = home_dir()?.display().to_string().replace('\\', "/");
    let agents = pi_agents_path()?;
    let existing = read_to_string_if_exists(&agents)?;
    write_text(&agents, &patch_pi_agents(&existing, &home))?;
    let skills_root = pi_dir.join("skills");
    let skills = write_entry_skills(&skills_root, true)?;
    let modules = seed_offline_modules(&skills_root)?;
    Ok(LskillApplyOutcome {
        message: format!(
            "已把破甲 1.5.9 接到 Pi（入口技能 {skills} 个，补齐深度模块 {modules} 个）"
        ),
        backup_id,
        codex_dir: pi_dir,
    })
}

pub(crate) fn clear_pi_lskill_159() -> Result<LskillApplyOutcome> {
    let pi_dir = pi_home_dir()?;
    if !pi_agents_file_is_lskill_159()? {
        return Err(CodexxError::Config("当前 Pi 不是破甲 1.5.9 离线通道".to_string()));
    }
    let backup_id = create_pi_prompt_backup("disable-lskill-1.5.9")?;
    let agents = pi_agents_path()?;
    let existing = read_to_string_if_exists(&agents)?;
    write_text(&agents, &strip_pi_lskill_block(&existing))?;
    Ok(LskillApplyOutcome {
        message: "已从 Pi AGENTS.md 撤下破甲 1.5.9 路由，入口技能文件已保留".to_string(),
        backup_id,
        codex_dir: pi_dir,
    })
}
