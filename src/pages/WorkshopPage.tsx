import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  BookOpen,
  ChevronDown,
  Braces,
  Download,
  FolderCog,
  Plus,
  RefreshCw,
  RotateCcw,
  Save,
  Shield,
  Trash2,
  Upload,
  Wrench,
  Zap,
} from "lucide-react";
import * as workshop from "../services/workshopService";
import type {
  BuiltinPromptStatus,
  ClaudePromptState,
  CodexPromptState,
  ManagedMcpServer,
  ManagedSkill,
  McpAllEngineReport,
  McpHostDiscovery,
  PiPromptState,
  PromptBackupEntry,
  SavedPrompt,
  SkillsMcpState,
} from "../types/workshop";
import "./WorkshopPage.css";

type Tab = "prompts" | "mcp" | "skills";
type Engine = "codex" | "claude" | "pi";
type InjectMode = "replace" | "append";

// MCP 自动接入目录（与 everything-patch 内置集成一致）
const MCP_DIRECTORY = [
  {
    id: "cheatengine-mcp",
    name: "Cheat Engine",
    description: "CE 内存分析 MCP：ce_mcp_bridge + mcp_cheatengine.py（Windows）",
    category: "逆向",
  },
  {
    id: "x64dbg-mcp",
    name: "x64dbg",
    description: "调试器 MCP：x64dbg.py + MCPx64dbg 插件（Windows）",
    category: "逆向",
  },
  {
    id: "burp-suite-mcp",
    name: "Burp Suite",
    description: "Web 安全 MCP：burp-mcp-all 扩展 + stdio 代理（Codex/Claude）",
    category: "Web 安全",
  },
  {
    id: "ida-pro-mcp",
    name: "IDA Pro",
    description: "IDA 静态分析 MCP：idalib-mcp（需要 uv）",
    category: "逆向",
  },
];

export default function WorkshopPage() {
  const [tab, setTab] = useState<Tab>("prompts");

  return (
    <div className="workshop-page">
      <div className="workshop-tabs">
        <button className={tab === "prompts" ? "is-active" : ""} onClick={() => setTab("prompts")}>
          <BookOpen size={15} /> 提示词
        </button>
        <button className={tab === "mcp" ? "is-active" : ""} onClick={() => setTab("mcp")}>
          <Braces size={15} /> MCP
        </button>
        <button className={tab === "skills" ? "is-active" : ""} onClick={() => setTab("skills")}>
          <Wrench size={15} /> Skills
        </button>
      </div>

      {tab === "prompts" && <PromptsTab />}
      {tab === "mcp" && <McpTab />}
      {tab === "skills" && <SkillsTab />}
    </div>
  );
}

// ---------------- 提示词 Tab ----------------

function PromptsTab() {
  const [engine, setEngine] = useState<Engine>("codex");
  const [mode, setMode] = useState<InjectMode>("replace");
  const [codexState, setCodexState] = useState<CodexPromptState | null>(null);
  const [claudeState, setClaudeState] = useState<ClaudePromptState | null>(null);
  const [piState, setPiState] = useState<PiPromptState | null>(null);
  const [builtins, setBuiltins] = useState<BuiltinPromptStatus[]>([]);
  const [saved, setSaved] = useState<SavedPrompt[]>([]);
  const [backups, setBackups] = useState<PromptBackupEntry[]>([]);
  const [busy, setBusy] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // 新建自定义提示词
  const [draftOpen, setDraftOpen] = useState(false);
  const [draftTitle, setDraftTitle] = useState("");
  const [draftFilename, setDraftFilename] = useState("");
  const [draftContent, setDraftContent] = useState("");
  const [draftRules, setDraftRules] = useState("");
  // 备份区默认折叠
  const [backupOpen, setBackupOpen] = useState(false);
  // 启用确认弹窗：写入前必须展示将改动的文件清单
  const [confirmAction, setConfirmAction] = useState<{
    kind: "builtin" | "saved";
    id: string;
    title: string;
    rules?: string | null;
    files: string[];
  } | null>(null);

  const reload = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const [b, s] = await Promise.all([
        workshop.listBuiltinPrompts(),
        workshop.listSavedPrompts(engine),
      ]);
      setBuiltins(b);
      setSaved(s);
      if (engine === "codex") {
        setCodexState(await workshop.getCodexPromptState());
        setBackups(await workshop.listPromptBackups("codex"));
      } else if (engine === "claude") {
        setClaudeState(await workshop.getClaudePromptState());
        setBackups(await workshop.listPromptBackups("claude"));
      } else {
        setPiState(await workshop.getPiPromptState());
        setBackups(await workshop.listPromptBackups("pi"));
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [engine]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const activeTitle = engine === "codex"
    ? (codexState?.instructionTemplateKey ?? null)
    : engine === "claude"
      ? (claudeState?.activeInstructionTitle ?? null)
      : (piState?.instructionTemplateKey ?? piState?.activeInstructionTitle ?? null);

  /** 计算本次启用将写入/替换的文件清单（确认弹窗展示用） */
  const affectedFiles = (id: string, filename: string, rules?: string | null): string[] => {
    const home = "C:\\Users\\jjwde";
    const codexDir = codexState?.codexDir || home + "\\.codex";
    const claudeDir = claudeState?.claudeDir || home + "\\.claude";
    const piDir = piState?.piDir || home + "\\.pi\\agent";
    if (engine === "codex") {
      if (id === "lskill-1.5.9") {
        return [
          `${codexDir}\\AGENTS.md  （整份替换为破甲 1.5.9 离线通道）`,
          `${codexDir}\\skills\\l-*\\SKILL.md  （写入 5 个入口技能）`,
          `${codexDir}\\skills\\_offline\\modules  （本地已有则保留，缺失则从 Downloads 补齐）`,
          `${codexDir}\\config.toml  （清除 model_instructions_file，避免盖掉通道）`,
        ];
      }
      const files =
        mode === "replace"
          ? [
              `${codexDir}\\config.toml  （设置 model_instructions_file = "./${filename}"）`,
              `${codexDir}\\${filename}  （写入提示词正文）`,
              `${codexDir}\\AGENTS.md  （移除受管区块，如有）`,
            ]
          : [`${codexDir}\\AGENTS.md  （追加受管区块，保留原提示词）`];
      if (rules) {
        files.push(`${codexDir}\\rules\\dc-${id}.rules  （写入配套 rules）`);
      }
      return files;
    }
    if (engine === "pi") {
      return [
        `${piDir}\\AGENTS.md  （写入破甲 1.5.9 离线路由表）`,
        `${piDir}\\skills\\l-*\\SKILL.md  （写入 5 个入口技能）`,
        `${piDir}\\skills\\_offline\\modules  （本地已有则保留，缺失则从 Downloads 补齐）`,
      ];
    }
    return [
      `${claudeDir}\\CLAUDE.md  （${mode === "replace" ? "替换" : "追加"}受管区块）`,
      `${claudeDir}\\keysmith\\${filename}  （写入提示词正文）`,
    ];
  };

  const handleEnableBuiltin = async (templateId: string) => {
    const meta = builtins.find(b => b.id === templateId);
    setConfirmAction({
      kind: "builtin",
      id: templateId,
      title: meta?.title ?? templateId,
      rules: meta?.hasRules ? "（内置配套 rules）" : null,
      files: affectedFiles(templateId, meta?.filename ?? `${templateId}.md`, meta?.hasRules ? "builtin" : null),
    });
  };

  const handleEnableSaved = async (id: string) => {
    const prompt = saved.find(p => p.id === id);
    setConfirmAction({
      kind: "saved",
      id,
      title: prompt?.title ?? id,
      rules: prompt?.rulesContent ?? null,
      files: affectedFiles(id, prompt?.filename ?? `${id}.md`, prompt?.rulesContent ?? null),
    });
  };

  /** 确认弹窗点"确认启用"后真正执行 */
  const executeEnable = async () => {
    if (!confirmAction) return;
    const { kind, id } = confirmAction;
    setConfirmAction(null);
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      if (kind === "builtin") {
        if (engine === "codex") {
          const r = await workshop.enableBuiltinPrompt(id, mode);
          setCodexState(r.state);
        } else if (engine === "claude") {
          const r = await workshop.enableClaudeBuiltin(id, mode);
          setClaudeState(r.state);
        } else {
          const r = await workshop.enablePiBuiltin(id, mode);
          setPiState(r.state);
        }
      } else {
        if (engine === "codex") {
          const r = await workshop.enableSavedPrompt(id, mode);
          setCodexState(r.state);
        } else if (engine === "claude") {
          const r = await workshop.enableClaudeSaved(id, mode);
          setClaudeState(r.state);
        } else {
          const r = await workshop.enablePiSaved(id, mode);
          setPiState(r.state);
        }
      }
      setNotice(`已启用：${confirmAction.title}`);
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDisable = async () => {
    setBusy(true);
    setError(null);
    try {
      if (engine === "codex") {
        const r = await workshop.disableInstruction(true);
        setCodexState(r.state);
      } else if (engine === "claude") {
        const r = await workshop.disableClaudeInstruction();
        setClaudeState(r.state);
      } else {
        const r = await workshop.disablePiInstruction();
        setPiState(r.state);
      }
      setNotice("已禁用");
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleSaveDraft = async () => {
    if (!draftTitle.trim() || !draftContent.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const filename =
        draftFilename.trim() ||
        draftTitle
          .trim()
          .toLowerCase()
          .replace(/[^\w\u4e00-\u9fa5]+/g, "-")
          .replace(/^-+|-+$/g, "")
          .slice(0, 48) + ".md";
      if (engine === "pi") {
        await workshop.savePiPrompt({
          id: crypto.randomUUID(),
          title: draftTitle.trim(),
          filename,
          content: draftContent,
          rulesContent: draftRules.trim() ? draftRules : null,
        });
      } else {
        await workshop.savePrompt({
          id: crypto.randomUUID(),
          title: draftTitle.trim(),
          filename,
          content: draftContent,
          rulesContent: draftRules.trim() ? draftRules : null,
        });
      }
      setDraftOpen(false);
      setDraftTitle("");
      setDraftFilename("");
      setDraftContent("");
      setDraftRules("");
      setNotice("已保存自定义提示词");
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDeleteSaved = async (id: string) => {
    await workshop.deleteSavedPrompt(id);
    setSaved(saved.filter((p) => p.id !== id));
  };

  const handleRestoreBackup = async (backupId: string) => {
    setBusy(true);
    try {
      await workshop.restorePromptBackup(engine, backupId);
      setNotice("已还原备份");
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };



  const stateBadge = useMemo(() => {
    if (engine === "codex") {
      if (!codexState) return "读取中…";
      if (codexState.instructionEnabled) return `已启用（${codexState.instructionInjectionMode ?? "replace"}）`;
      if (codexState.instructionStatus === "external") return "外部提示词（读取中）";
      if (codexState.instructionStatus === "inactive") return "有未加载的受管文件";
      return "未启用";
    }
    if (engine === "pi") {
      if (!piState) return "读取中…";
      return piState.instructionEnabled ? "已启用破甲 1.5.9" : "未启用";
    }
    if (!claudeState) return "读取中…";
    return claudeState.instructionEnabled ? "已启用" : "未启用";
  }, [engine, codexState, claudeState, piState]);

  return (
    <div className="workshop-tab-body">
      {error && <div className="workshop-error">{error}</div>}
      {notice && <div className="workshop-notice">{notice}</div>}

      <div className="workshop-toolbar">
        <label>目标引擎</label>
        <button
          className={`btn btn-sm ${engine === "codex" ? "btn-primary" : "btn-secondary"}`}
          onClick={() => setEngine("codex")}
        >
          Codex
        </button>
        <button
          className={`btn btn-sm ${engine === "claude" ? "btn-primary" : "btn-secondary"}`}
          onClick={() => setEngine("claude")}
        >
          Claude Code
        </button>
        <button
          className={`btn btn-sm ${engine === "pi" ? "btn-primary" : "btn-secondary"}`}
          onClick={() => setEngine("pi")}
        >
          Pi
        </button>
        <span className="workshop-separator" />
        <label>启用方式</label>
        <button
          className={`btn btn-sm ${mode === "replace" ? "btn-primary" : "btn-secondary"}`}
          onClick={() => setMode("replace")}
          title="写入 model_instructions_file（Codex）或替换受管区块"
        >
          替换原提示词
        </button>
        <button
          className={`btn btn-sm ${mode === "append" ? "btn-primary" : "btn-secondary"}`}
          onClick={() => setMode("append")}
          title="保留原提示词，追加受管区块"
        >
          保留原提示词
        </button>
        <span className="workshop-separator" />
        <button className="btn btn-sm btn-secondary" onClick={() => void reload()} disabled={busy}>
          <RefreshCw size={13} /> 刷新
        </button>
        <button className="btn btn-sm btn-secondary" onClick={() => setDraftOpen(true)}>
          <Plus size={13} /> 新建自定义
        </button>
      </div>

      <div className="workshop-status-card">
        <Shield size={16} />
        <div>
          <div className="ws-card-title">
            当前状态（{engine === "codex" ? "Codex" : engine === "claude" ? "Claude" : "Pi"}）：{stateBadge}
          </div>
          <div className="ws-card-sub">
            {engine === "codex"
              ? codexState
                ? `Codex 目录：${codexState.codexDir} │ 指令文件：${codexState.instructionFile ?? "无"}`
                : "读取中…"
              : engine === "pi"
                ? piState
                  ? `Pi 目录：${piState.piDir}`
                  : "读取中…"
                : claudeState
                  ? `Claude 目录：${claudeState.claudeDir}`
                  : "读取中…"}
          </div>
        </div>
        <button className="btn btn-sm btn-secondary" onClick={() => void handleDisable()} disabled={busy}>
          <Trash2 size={13} /> 禁用
        </button>
      </div>

      <div className="workshop-section">
        <div className="workshop-section-title">
          内置模板
          {activeTitle && (
            <span className="ws-current">
              （当前：{builtins.find(b => `builtin:${b.id}` === activeTitle)?.title
                ?? saved.find(s => `saved:${s.id}` === activeTitle)?.title
                ?? activeTitle}）
            </span>
          )}
        </div>
        <div className="workshop-card-grid">
          {(engine === "pi"
            ? builtins.filter((p) => p.id === "lskill-1.5.9")
            : builtins.filter((p) =>
                engine === "codex"
                  ? !p.badge?.includes("Anthropic")
                  : engine === "claude"
                    ? !p.badge?.includes("OpenAI")
                    : true
              )
          ).map((p) => {
            const isActive = activeTitle === `builtin:${p.id}`;
            return (
            <div className={`workshop-card ${isActive ? "is-active" : ""}`} key={p.id}>
              <div className="workshop-card-head">
                <strong className="ws-card-title">{p.title}</strong>
                <span className="ws-badge-group">
                  {isActive && <span className="ws-badge ws-badge-on">已启用</span>}
                  {p.hasRules && <span className="ws-badge ws-badge-rules">RULES</span>}
                  {p.badge && <span className="ws-badge">{p.badge}</span>}
                </span>
              </div>
              {p.subtitle && <div className="ws-card-sub">{p.subtitle}</div>}
              <div className="ws-card-foot">
                <button
                  className={`btn btn-sm ${isActive ? "btn-primary" : "btn-secondary"}`}
                  disabled={busy || isActive}
                  onClick={() => void handleEnableBuiltin(p.id)}
                >
                  <Zap size={13} /> {isActive ? "已启用" : "启用"}
                </button>
                <span className="ws-source">{p.message}</span>
              </div>
            </div>
            );
          })}
        </div>
      </div>

      <div className="workshop-section">
        <div className="workshop-section-title">自定义提示词（{saved.length}）</div>
        <div className="workshop-card-grid">
          {[...saved]
            .sort((a, b) => Number(!!b.rulesContent) - Number(!!a.rulesContent))
            .map((p) => (
            <div className="workshop-card" key={p.id}>
              <div className="workshop-card-head">
                <strong>{p.title}</strong>
                {p.rulesContent ? <span className="ws-badge ws-badge-rules">RULES</span> : <span className="ws-filename">{p.filename}</span>}
              </div>
              <div className="ws-card-foot">
                <button
                  className="btn btn-sm btn-primary"
                  disabled={busy}
                  onClick={() => void handleEnableSaved(p.id)}
                >
                  <Zap size={13} /> 启用
                </button>
                <button
                  className="btn btn-sm btn-secondary"
                  onClick={() => void handleDeleteSaved(p.id)}
                >
                  <Trash2 size={13} /> 删除
                </button>
              </div>
            </div>
          ))}
          {saved.length === 0 && <div className="ws-empty">暂无自定义提示词</div>}
        </div>
      </div>

      <div className="workshop-section ws-backup-panel">
        <button className="ws-backup-header" onClick={() => setBackupOpen(v => !v)}>
          <ChevronDown size={14} className={`ws-chevron ${backupOpen ? "is-open" : ""}`} />
          <span>备份快照（{backups.length}）</span>
          <span className="ws-backup-hint">每次启用 / 禁用前自动留档，可随时还原</span>
        </button>
        {backupOpen && (
          <div className="workshop-backup-list">
            {backups.map((b) => (
              <div className="ws-backup-row" key={b.id}>
                <span className="ws-backup-time">{b.createdAt.replace("T", " ").slice(0, 19)}</span>
                <span className="ws-badge">{b.action}</span>
                <span className="ws-backup-id">{b.id}</span>
                <button className="btn btn-sm btn-secondary" onClick={() => void handleRestoreBackup(b.id)}>
                  <RotateCcw size={13} /> 还原
                </button>
              </div>
            ))}
            {backups.length === 0 && <div className="ws-empty">暂无备份</div>}
          </div>
        )}
      </div>

      <div className="ws-footer">
        <span>数据目录 ~/.cockpit/devconduit.db</span>
        <span className="ws-footer-dot">·</span>
        <span>写入目标 {engine === "codex" ? "~/.codex" : engine === "pi" ? "~/.pi/agent" : "~/.claude"}</span>
        <span>启用前自动快照</span>
      </div>

      {confirmAction && (
        <div className="workshop-modal">
          <div className="workshop-modal-box">
            <h3>确认启用：{confirmAction.title}</h3>
            <div className="ws-card-sub">
              引擎：{engine === "codex" ? "Codex" : engine === "claude" ? "Claude Code" : "Pi"} · 模式：{mode === "replace" ? "替换原提示词" : "保留原提示词（追加区块）"}
            </div>
            <label>本次将写入 / 替换以下文件（写入前已自动快照备份）：</label>
            <div className="ws-confirm-files">
              {confirmAction.files.map((f) => (
                <div className="ws-confirm-file" key={f}>{f}</div>
              ))}
            </div>
            {confirmAction.rules && (
              <div className="ws-card-sub">该提示词带配套 rules（{confirmAction.rules.length} 字符），将同步写入 rules 目录。</div>
            )}
            <div className="workshop-modal-actions">
              <button className="btn btn-sm btn-secondary" onClick={() => setConfirmAction(null)}>取消</button>
              <button className="btn btn-sm btn-primary" onClick={() => void executeEnable()}>
                <Zap size={13} /> 确认启用
              </button>
            </div>
          </div>
        </div>
      )}

      {draftOpen && (
        <div className="workshop-modal">
          <div className="workshop-modal-box">
            <h3>新建自定义提示词</h3>
            <label>标题</label>
            <input value={draftTitle} onChange={(e) => setDraftTitle(e.target.value)} placeholder="如：无限破甲 v3" />
            <label>文件名（.md，可留空自动生成）</label>
            <input value={draftFilename} onChange={(e) => setDraftFilename(e.target.value)} placeholder="unlimited-break.md" />
            <label>内容</label>
            <textarea rows={10} value={draftContent} onChange={(e) => setDraftContent(e.target.value)} placeholder="提示词正文…" />
            <label>配套 rules（可选，启用时写入 ~/.codex/rules/dc-&lt;id&gt;.rules）</label>
            <textarea rows={5} value={draftRules} onChange={(e) => setDraftRules(e.target.value)} placeholder="# Starlark 规则文本，留空则不写 rules" />
            <div className="workshop-modal-actions">
              <button className="btn btn-sm btn-secondary" onClick={() => setDraftOpen(false)}>取消</button>
              <button className="btn btn-sm btn-primary" onClick={() => void handleSaveDraft()} disabled={busy}>
                <Save size={13} /> 保存
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------- MCP Tab ----------------

function McpTab() {
  // FORK: MCP 全局化 —— 不再按引擎各管一套。安装/启停/删除一次性作用于全部引擎，
  // 列表为跨引擎聚合视图（同一 server 只出现一行，标注已装引擎）。
  const ENGINE_TOOLS: Array<{ id: Engine; label: string }> = [
    { id: "codex", label: "Codex" },
    { id: "claude", label: "Claude Code" },
    { id: "pi", label: "Pi" },
  ];
  const [statesByTool, setStatesByTool] = useState<Partial<Record<Engine, SkillsMcpState>>>({});
  const [hosts, setHosts] = useState<McpHostDiscovery[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const [codex, claude, pi, h] = await Promise.all([
        workshop.getSkillsMcpState("codex"),
        workshop.getSkillsMcpState("claude"),
        workshop.getSkillsMcpState("pi"),
        workshop.discoverMcpHosts(),
      ]);
      setStatesByTool({ codex, claude, pi });
      setHosts(h);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  const formatReports = (reports: McpAllEngineReport[]) => {
    const okParts = reports.filter((r) => r.ok).map((r) => `${r.toolLabel}（${r.message}）`);
    const failParts = reports.filter((r) => !r.ok).map((r) => `${r.toolLabel}：${r.message}`);
    const lines: string[] = [];
    if (okParts.length > 0) lines.push(`成功：${okParts.join("、")}`);
    if (failParts.length > 0) lines.push(`失败：${failParts.join("、")}`);
    return lines.join("\n");
  };

  const handleToggleAll = async (id: string, enabled: boolean) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const reports = await workshop.toggleMcpAll(id, enabled);
      setNotice(`已${enabled ? "启用" : "停用"}全部引擎。\n${formatReports(reports)}`);
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleDeleteAll = async (id: string) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const reports = await workshop.uninstallMcpAll(id);
      setNotice(`已从全部引擎删除。\n${formatReports(reports)}`);
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleInstallAll = async (integrationId: string) => {
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const reports = await workshop.installMcpIntegrationAll({
        integrationId,
        sourceMode: "managed",
        autoInstall: true,
      });
      setNotice(`安装完成。\n${formatReports(reports)}`);
      await reload();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const hostDetectedFor = (integrationId: string) => {
    const d = hosts.find((h) => h.integrationId === integrationId);
    if (!d || d.candidates.length === 0) return null;
    return d.candidates[0];
  };

  // 聚合三个引擎的 server：id 相同视为同一 MCP。
  const aggregated: Array<{ server: ManagedMcpServer; tools: string[]; anyEnabled: boolean }> = (() => {
    const byId = new Map<string, { server: ManagedMcpServer; tools: string[]; anyEnabled: boolean }>();
    for (const { id, label } of ENGINE_TOOLS) {
      for (const server of statesByTool[id]?.mcpServers ?? []) {
        const entry = byId.get(server.id) ?? { server, tools: [], anyEnabled: false };
        if (!entry.tools.includes(label)) entry.tools.push(label);
        entry.anyEnabled = entry.anyEnabled || server.enabled;
        if (server.enabled) entry.server = server;
        byId.set(server.id, entry);
      }
    }
    return Array.from(byId.values()).sort((a, b) => a.server.name.localeCompare(b.server.name));
  })();

  const available = MCP_DIRECTORY.filter(
    (m) => !aggregated.some((a) => a.server.id === m.id || a.server.summary.includes(m.name)),
  );

  return (
    <div className="workshop-tab-body">
      {error && <div className="workshop-error">{error}</div>}
      {notice && <div className="workshop-notice workshop-notice-pre">{notice}</div>}
      <div className="workshop-toolbar">
        <span className="ws-card-sub">安装 / 启停 / 删除会同时作用于全部引擎（Codex、Claude Code、Pi 等）</span>
        <button className="btn btn-sm btn-secondary" onClick={() => void reload()} disabled={busy}><RefreshCw size={13} /> 刷新</button>
      </div>

      <div className="workshop-section">
        <div className="workshop-section-title">已挂载 MCP Server（{aggregated.length}）</div>
        <div className="workshop-mcp-list">
          {aggregated.map(({ server, tools, anyEnabled }) => (
            <div className="ws-mcp-row" key={server.id}>
              <div className="ws-mcp-main">
                <strong>{server.name}</strong>
                <span className="ws-source">{server.transport}{anyEnabled ? " · 已启用" : " · 已停用"}</span>
                <span className="ws-badge">{tools.join(" / ") || "未挂载到任何引擎"}</span>
                {server.summary && <div className="ws-card-sub">{server.summary}</div>}
                {server.command && <div className="ws-mono">{server.command}</div>}
                {server.url && <div className="ws-mono">{server.url}</div>}
              </div>
              <div className="ws-mcp-actions">
                <button className={`btn btn-sm ${anyEnabled ? "btn-secondary" : "btn-primary"}`} disabled={busy}
                  onClick={() => void handleToggleAll(server.id, !anyEnabled)}>
                  {anyEnabled ? "全部停用" : "全部启用"}
                </button>
                <button className="btn btn-sm btn-secondary" disabled={busy} onClick={() => void handleDeleteAll(server.id)}>
                  <Trash2 size={13} /> 删除（全部引擎）
                </button>
              </div>
            </div>
          ))}
          {aggregated.length === 0 && <div className="ws-empty">尚未挂载 MCP</div>}
        </div>
      </div>

      <div className="workshop-section">
        <div className="workshop-section-title">自动接入目录</div>
        <div className="workshop-card-grid">
          {available.map((m) => {
            const host = hostDetectedFor(m.id);
            return (
              <div className="workshop-card" key={m.id}>
                <div className="workshop-card-head">
                  <strong>{m.name}</strong>
                  <span className="ws-badge">{m.category}</span>
                </div>
                <div className="ws-card-sub">{m.description}</div>
                {host ? (
                  <div className="ws-host">已检测到宿主：{host.path}</div>
                ) : (
                  <div className="ws-card-sub">未检测到宿主（可手动配置）</div>
                )}
                <div className="ws-card-foot">
                  <button className="btn btn-sm btn-primary" disabled={busy} onClick={() => void handleInstallAll(m.id)}>
                    <FolderCog size={13} /> 安装到全部引擎
                  </button>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

// ---------------- Skills Tab ----------------}

// ---------------- Skills Tab ----------------}

// ---------------- Skills Tab ----------------

function SkillsTab() {
  const [tool, setTool] = useState<Engine>("codex");
  const [state, setState] = useState<SkillsMcpState | null>(null);
  const [preview, setPreview] = useState<{ skills: ManagedSkill[] } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const fileRef = useRef<HTMLInputElement>(null);

  const reload = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      setState(await workshop.getSkillsMcpState(tool));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }, [tool]);

  useEffect(() => {
    void reload();
  }, [reload]);

  const filteredSkills = useMemo(() => {
    const skills = state?.skills ?? [];
    const needle = query.trim().toLowerCase();
    if (!needle) return skills;
    return skills.filter((skill) =>
      [skill.name, skill.description ?? "", skill.id, skill.path]
        .join(" ")
        .toLowerCase()
        .includes(needle),
    );
  }, [query, state?.skills]);
  const handleToggle = async (skill: ManagedSkill, enabled: boolean) => {
    setState(await workshop.toggleSkill(tool, skill.id, enabled));
  };

  const handleUninstall = async (skill: ManagedSkill) => {
    setBusy(true);
    try {
      const r = await workshop.uninstallSkill(tool, skill.id);
      setNotice(r.message);
      setState(r.state);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handlePreview = async () => {
    setBusy(true);
    try {
      const p = await workshop.previewToolImport(tool);
      setPreview(p);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleImport = async () => {
    setBusy(true);
    try {
      const r = await workshop.importExistingSkills(tool);
      setNotice(r.message);
      setPreview(null);
      setState(r.state);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleCheckUpdates = async () => {
    setBusy(true);
    try {
      setState(await workshop.checkSkillUpdates(tool));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  // FORK: 寒霜 breaker-kit 技能包一键安装（内置 zip，无需本地文件）。
  const [kitInstalling, setKitInstalling] = useState(false);
  const handleInstallBreakerKit = async () => {
    setKitInstalling(true);
    setError(null);
    setNotice(null);
    try {
      const r = await workshop.installBreakerKit(tool);
      setNotice(r.message);
      setState(r.state);
    } catch (e) {
      setError(String(e));
    } finally {
      setKitInstalling(false);
    }
  };

  const handleZipFile = async (file: File | null) => {
    if (!file) return;
    if (!file.name.toLowerCase().endsWith(".zip")) {
      setError("只支持 .zip 文件");
      return;
    }
    if (file.size > 20 * 1024 * 1024) {
      setError("ZIP 不能超过 20MB");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const bytes = Array.from(new Uint8Array(await file.arrayBuffer()));
      const r = await workshop.installSkillZip(tool, file.name, bytes);
      setNotice(r.message);
      setState(r.state);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
      if (fileRef.current) fileRef.current.value = "";
    }
  };

  return (
    <div className="workshop-tab-body">
      {error && <div className="workshop-error">{error}</div>}
      {notice && <div className="workshop-notice">{notice}</div>}
      <div className="workshop-toolbar">
        <label>目标引擎</label>
        <button className={`btn btn-sm ${tool === "codex" ? "btn-primary" : "btn-secondary"}`} onClick={() => setTool("codex")}>Codex</button>
        <button className={`btn btn-sm ${tool === "claude" ? "btn-primary" : "btn-secondary"}`} onClick={() => setTool("claude")}>Claude Code</button>
        <button className={`btn btn-sm ${tool === "pi" ? "btn-primary" : "btn-secondary"}`} onClick={() => setTool("pi")}>Pi</button>
        <button className="btn btn-sm btn-secondary" onClick={() => void reload()} disabled={busy}><RefreshCw size={13} /> 刷新</button>
        <button className="btn btn-sm btn-secondary" onClick={() => void handlePreview()} disabled={busy}><Upload size={13} /> 扫描可导入</button>
        <button className="btn btn-sm btn-secondary" onClick={() => void handleImport()} disabled={busy}><Download size={13} /> 导入已有</button>
        <button className="btn btn-sm btn-secondary" onClick={() => void handleCheckUpdates()} disabled={busy}><RefreshCw size={13} /> 检查更新</button>
        <button className="btn btn-sm btn-primary" onClick={() => fileRef.current?.click()}><FolderCog size={13} /> 安装 ZIP</button>
        <button
          className="btn btn-sm btn-primary"
          disabled={busy || kitInstalling}
          onClick={() => void handleInstallBreakerKit()}
          title="安装寒霜 breaker-kit 技能包（107 技能 + RULES + 脚本，解压到 ~/.codex/skills/hanshuang-breaker-kit/）"
        >
          {kitInstalling ? <RefreshCw size={13} className="loading-spinner" /> : <FolderCog size={13} />}
          寒霜技能包
        </button>
        <input
          className="ws-search"
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          placeholder="搜索技能名 / 描述 / 文件，例如 anti-debug、证书、hook"
        />
        <input ref={fileRef} type="file" accept=".zip" hidden onChange={(e) => void handleZipFile(e.target.files?.[0] ?? null)} />
      </div>
      {preview && (
        <div className="workshop-section">
          <div className="workshop-section-title">可导入的 Skills（{preview.skills.length}）</div>
          {preview.skills.map((s) => (
            <div className="ws-backup-row" key={s.path}>
              <span>{s.name}</span>
              <span className="ws-source">{s.path}</span>
            </div>
          ))}
        </div>
      )}

      <div className="workshop-section">
        <div className="workshop-section-title">当前 Skills（{filteredSkills.length}/{state?.skills.length ?? 0}）</div>
        <div className="workshop-card-grid">
          {filteredSkills.map((s) => (
            <div className="workshop-card" key={s.id}>
              <div className="workshop-card-head">
                <strong>{s.name}</strong>
                {s.updateStatus && s.updateStatus !== "none" && <span className="ws-badge ws-badge-warn">{s.updateStatus}</span>}
              </div>
              {s.description ? <div className="ws-card-sub">{s.description}</div> : <div className="ws-card-sub">{s.path}</div>}
              <div className="ws-card-foot">
                <button className={`btn btn-sm ${s.enabled ? "btn-secondary" : "btn-primary"}`} disabled={busy} onClick={() => void handleToggle(s, !s.enabled)}>
                  {s.enabled ? "已启用 · 停用" : "启用"}
                </button>
                <button className="btn btn-sm btn-secondary" disabled={busy} onClick={() => void handleUninstall(s)}>
                  <Trash2 size={13} /> 卸载
                </button>
              </div>
            </div>
          ))}
          {(state?.skills ?? []).length === 0 && <div className="ws-empty">尚未发现 Skills</div>}
        </div>
      </div>
    </div>
  );
}
