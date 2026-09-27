import { useCallback, useEffect, useRef, useState } from "react";
import { Network, Plus, RotateCcw, Save, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import * as codexLocalAccessService from "../services/codexLocalAccessService";
import type {
  CodexLocalAccessToolProfile,
  CodexLocalAccessToolRouterState,
} from "../types/codexLocalAccess";

/**
 * 工具路由面板。
 *
 * 它决定「这次请求把哪些 MCP 工具摆到模型面前, 以及用哪个模型」。
 *
 * 一个必须说清的能力边界: proxy 是 HTTP 中继, **不执行工具**。真正的工具在
 * 客户端本地跑, 这里只能"少摆出来", 不能"凭空变出来"。所以档案里的白名单
 * 是收窄用的, 不是安装器 —— 客户端没注册的 MCP 工具无法靠这里获得。
 */

/**
 * 出厂默认配置, 与 Go 侧 defaultToolRouterConfig 保持一致。
 *
 * 五分类与仓库既有的 lskill 体系对齐(l-reverse / l-gameassist / l-webrecon /
 * l-license), 保证同一句话在提示词路由与工具路由落到同一类。
 * crawler 是额外的通用类, 本地无对应 lskill。
 *
 * 白名单关键词来源已核实, 不是猜的:
 *  - 仓库自带 MCP 集成写入 Codex 的 id: ida-pro-mcp / cheatengine-mcp /
 *    x64dbg-mcp / burp-suite-mcp (见 skills_mcp/catalog.rs), 命名空间即
 *    mcp__<id>__<tool>。
 *  - GitHub MCP 生态里实际存在的同类 server(经 awesome-mcp-servers 核对)。
 *
 * 注意: 公开生态里没有 CheatEngine / x64dbg / 破解类的通用 MCP server,
 * 只有仓库自带的两个。所以 gameassist 与 license 的覆盖面天然更窄。
 */
function defaultState(): CodexLocalAccessToolRouterState {
  return {
    enabled: true,
    modelRouting: false,
    defaultProfile: "default",
    profiles: [
      {
        id: "default",
        label: "默认(不裁剪)",
        allowNamespaces: [],
        denyNamespaces: [],
        model: "",
      },
      {
        id: "reverse",
        label: "二进制逆向",
        allowNamespaces: [
          "ida-pro", "ida-headless", "ida", "ghidra", "radare", "r2mcp",
          "binary_ninja", "binaryninja", "binja", "lldb", "solvitor",
          "apktool", "jadx", "objdump", "decompile",
        ],
        denyNamespaces: ["crawl", "scrap", "playwright"],
        model: "",
      },
      {
        id: "gameassist",
        label: "游戏辅助",
        allowNamespaces: [
          "cheatengine", "cheat-engine", "cheat_engine", "x64dbg", "x32dbg",
          "frida", "il2cpp", "ue5", "unreal", "unity",
          "pine", "bizhawk", "dolphin", "mgba", "pcsx2", "emulator",
          "ida", "ghidra",
        ],
        denyNamespaces: ["crawl", "scrap"],
        model: "",
      },
      {
        id: "webrecon",
        label: "Web 渗透",
        allowNamespaces: [
          "burp", "zap", "nuclei", "sqlmap", "nmap", "masscan",
          "secops", "sast", "dast", "wireshark", "cybersec",
          "httpx", "ffuf",
        ],
        denyNamespaces: ["crawl", "scrap"],
        model: "",
      },
      {
        id: "license",
        label: "授权破解",
        allowNamespaces: [
          "ida", "ghidra", "radare", "r2mcp", "binary_ninja", "binja",
          "x64dbg", "x32dbg", "cheatengine", "lldb", "frida",
          "apktool", "jadx", "decompile",
        ],
        denyNamespaces: ["crawl", "scrap"],
        model: "",
      },
      {
        id: "crawler",
        label: "爬虫采集",
        allowNamespaces: [
          "crawl", "scrap", "spider", "fetch", "webreaper", "pyrecrawl",
          "browser", "playwright", "puppeteer", "selenium", "browserless",
          "browser-use", "firecrawl", "headless", "nodriver",
        ],
        denyNamespaces: ["ida", "ghidra", "x64dbg", "cheatengine", "burp"],
        model: "",
      },
    ],
    rules: [
      { class: "reverse", weight: 10, pattern: "逆向|反编译|脱壳|反汇编|逆向工程|动态调试|静态分析" },
      { class: "reverse", weight: 9, pattern: "IDA|Ghidra|radare2|rizin|Binary Ninja|lldb|objdump" },
      { class: "reverse", weight: 8, pattern: "apk|jadx|apktool|smali" },
      { class: "gameassist", weight: 10, pattern: "外挂|自瞄|透视|ESP|游戏辅助|游戏脚本|游戏内存|修改器" },
      { class: "gameassist", weight: 10, pattern: "内存修改|内存读写|内存搜索|读写内存|指针扫描|特征码|基址|偏移" },
      { class: "gameassist", weight: 9, pattern: "反作弊|反外挂|隐藏进程|驱动隐藏|过检测" },
      { class: "gameassist", weight: 9, pattern: "封包|改包|发包|服务器绕过|GM指令|GM面板|游戏协议" },
      { class: "license", weight: 11, pattern: "破解|绕过激活|去激活|去验证|移除授权|授权校验" },
      { class: "license", weight: 10, pattern: "keygen|注册机|卡密|序列号|激活码|授权码" },
      { class: "webrecon", weight: 10, pattern: "渗透|打点|漏洞扫描|漏扫|渗透测试|测安全" },
      { class: "webrecon", weight: 9, pattern: "sql注入|xss|ssrf|csrf|命令注入|反序列化" },
      { class: "webrecon", weight: 9, pattern: "越权|IDOR|权限提升|提权|认证绕过|加管理员" },
      { class: "crawler", weight: 10, pattern: "爬虫|爬取|采集|抓取|批量下载|数据抓取" },
      { class: "crawler", weight: 9, pattern: "scrapy|selenium|playwright|puppeteer|无头浏览器" },
    ],
  };
}

const PROFILE_IDS = ["default", "reverse", "gameassist", "webrecon", "license", "crawler"];

export function CodexLocalAccessToolRouterPanel() {
  const { t } = useTranslation();
  const [state, setState] = useState<CodexLocalAccessToolRouterState | null>(null);
  const [dirty, setDirty] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const load = useCallback(async () => {
    try {
      const loaded = await codexLocalAccessService.getCodexLocalAccessToolRouterState();
      if (!mountedRef.current) return;
      // 未初始化时用出厂默认填充, 让面板一开始就可编辑。
      setState(loaded ?? defaultState());
      setDirty(false);
      setError(null);
    } catch (err) {
      if (mountedRef.current) setError(String(err));
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const patch = useCallback((updater: (draft: CodexLocalAccessToolRouterState) => void) => {
    setState((current) => {
      if (!current) return current;
      // 深拷贝一层再改, 避免直接改到既有引用。
      const draft: CodexLocalAccessToolRouterState = {
        ...current,
        profiles: current.profiles.map((profile) => ({
          ...profile,
          allowNamespaces: [...profile.allowNamespaces],
          denyNamespaces: [...profile.denyNamespaces],
        })),
        rules: current.rules.map((rule) => ({ ...rule })),
      };
      updater(draft);
      return draft;
    });
    setDirty(true);
    setNotice(null);
  }, []);

  const save = useCallback(async () => {
    if (!state) return;
    setBusy(true);
    setError(null);
    try {
      const saved = await codexLocalAccessService.updateCodexLocalAccessToolRouter(state);
      if (!mountedRef.current) return;
      setState(saved);
      setDirty(false);
      setNotice(
        t("codex.localAccess.toolRouterSaved", "已保存, sidecar 将在 3 秒内热加载生效"),
      );
    } catch (err) {
      if (mountedRef.current) setError(String(err));
    } finally {
      if (mountedRef.current) setBusy(false);
    }
  }, [state, t]);

  const toggleEnabled = useCallback(async () => {
    if (!state) return;
    setBusy(true);
    setError(null);
    try {
      // 先保存当前编辑, 再切开关, 避免开关操作把未保存的编辑丢掉。
      if (dirty) {
        await codexLocalAccessService.updateCodexLocalAccessToolRouter(state);
      }
      const next = await codexLocalAccessService.updateCodexLocalAccessToolRouterEnabled(
        !state.enabled,
      );
      if (!mountedRef.current) return;
      setState(next);
      setDirty(false);
      setNotice(null);
    } catch (err) {
      if (mountedRef.current) setError(String(err));
    } finally {
      if (mountedRef.current) setBusy(false);
    }
  }, [state, dirty]);

  const reset = useCallback(() => {
    setState(defaultState());
    setDirty(true);
    setNotice(t("codex.localAccess.toolRouterReset", "已恢复出厂配置, 记得保存"));
  }, [t]);

  if (!state) {
    return (
      <div className="codex-local-access-tool-router" style={{ fontSize: 12, opacity: 0.75 }}>
        {t("common.loading", "加载中...")}
      </div>
    );
  }

  return (
    <div className="codex-local-access-tool-router" style={{ display: "grid", gap: 10 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 8, flexWrap: "wrap" }}>
        <Network size={14} />
        <strong style={{ fontSize: 12 }}>
          {t("codex.localAccess.toolRouterTitle", "工具路由")}
        </strong>
        <button
          type="button"
          className={`btn ${state.enabled ? "btn-primary" : "btn-secondary"}`}
          style={{ padding: "2px 8px", fontSize: 11 }}
          disabled={busy}
          onClick={() => void toggleEnabled()}
        >
          {state.enabled ? t("common.enabled", "已开启") : t("common.disabled", "已关闭")}
        </button>
        <label style={{ display: "flex", alignItems: "center", gap: 4, fontSize: 11 }}>
          <input
            type="checkbox"
            checked={state.modelRouting}
            disabled={busy}
            onChange={(event) =>
              patch((draft) => {
                draft.modelRouting = event.target.checked;
              })
            }
          />
          {t("codex.localAccess.toolRouterModelRouting", "按任务类型切换模型")}
        </label>
        <span style={{ flex: 1 }} />
        <button
          type="button"
          className="btn btn-secondary"
          style={{ padding: "2px 8px", fontSize: 11 }}
          disabled={busy}
          onClick={reset}
        >
          <RotateCcw size={12} /> {t("common.reset", "重置")}
        </button>
        <button
          type="button"
          className="btn btn-primary"
          style={{ padding: "2px 8px", fontSize: 11 }}
          disabled={busy || !dirty}
          onClick={() => void save()}
        >
          <Save size={12} /> {t("common.save", "保存")}
        </button>
      </div>

      <div style={{ fontSize: 11, opacity: 0.72, lineHeight: 1.5 }}>
        {t(
          "codex.localAccess.toolRouterHint",
          "按任务类型收窄要摆给模型的 MCP 工具(可另按类型换模型)。只裁剪 MCP 命名空间, 原生工具(shell/apply_patch 等)永不裁剪; 全放行的档案表示不裁剪。",
        )}
      </div>

      {/* 任务档案 */}
      <div style={{ display: "grid", gap: 8 }}>
        {state.profiles.map((profile, index) => (
          <ProfileEditor
            key={`${profile.id}-${index}`}
            profile={profile}
            disabled={busy}
            isDefault={profile.id === state.defaultProfile}
            onChange={(next) =>
              patch((draft) => {
                draft.profiles[index] = next;
              })
            }
          />
        ))}
      </div>

      {/* 分类规则 */}
      <div style={{ display: "grid", gap: 6 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <strong style={{ fontSize: 12 }}>
            {t("codex.localAccess.toolRouterRules", "分类规则")}
          </strong>
          <span style={{ fontSize: 11, opacity: 0.7 }}>
            {t("codex.localAccess.toolRouterRulesHint", "正则匹配用户消息, 同类权重相加取最高")}
          </span>
          <span style={{ flex: 1 }} />
          <button
            type="button"
            className="btn btn-secondary"
            style={{ padding: "2px 8px", fontSize: 11 }}
            disabled={busy}
            onClick={() =>
              patch((draft) => {
                draft.rules.push({ class: "reverse", pattern: "", weight: 5 });
              })
            }
          >
            <Plus size={12} /> {t("common.add", "新增")}
          </button>
        </div>

        <div style={{ display: "grid", gap: 4, maxHeight: 220, overflowY: "auto" }}>
          {state.rules.map((rule, index) => (
            <div key={index} style={{ display: "flex", gap: 4, alignItems: "center" }}>
              <select
                value={rule.class}
                disabled={busy}
                style={{ fontSize: 11, width: 96 }}
                onChange={(event) =>
                  patch((draft) => {
                    draft.rules[index].class = event.target.value;
                  })
                }
              >
                {PROFILE_IDS.map((id) => (
                  <option key={id} value={id}>
                    {id}
                  </option>
                ))}
              </select>
              <input
                type="number"
                value={rule.weight}
                disabled={busy}
                style={{ fontSize: 11, width: 56 }}
                onChange={(event) =>
                  patch((draft) => {
                    draft.rules[index].weight = Number(event.target.value) || 0;
                  })
                }
              />
              <input
                type="text"
                value={rule.pattern}
                disabled={busy}
                placeholder={t("codex.localAccess.toolRouterPattern", "正则, 如 逆向|反编译")}
                style={{ fontSize: 11, flex: 1, minWidth: 120 }}
                onChange={(event) =>
                  patch((draft) => {
                    draft.rules[index].pattern = event.target.value;
                  })
                }
              />
              <button
                type="button"
                className="btn btn-secondary"
                style={{ padding: "2px 6px", fontSize: 11 }}
                disabled={busy}
                onClick={() =>
                  patch((draft) => {
                    draft.rules.splice(index, 1);
                  })
                }
              >
                <Trash2 size={12} />
              </button>
            </div>
          ))}
        </div>
      </div>

      {notice && <div style={{ fontSize: 11, color: "var(--success, #4ec9a0)" }}>{notice}</div>}
      {error && <div style={{ fontSize: 11, color: "var(--danger, #f0616d)" }}>{error}</div>}
    </div>
  );
}

/** 单个任务档案的编辑器: 白名单 / 黑名单 / 目标模型。 */
function ProfileEditor({
  profile,
  disabled,
  isDefault,
  onChange,
}: {
  profile: CodexLocalAccessToolProfile;
  disabled: boolean;
  isDefault: boolean;
  onChange: (next: CodexLocalAccessToolProfile) => void;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);

  return (
    <div
      style={{
        border: "1px solid var(--border, #33373f)",
        borderRadius: 8,
        padding: 8,
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <button
          type="button"
          className="btn btn-secondary"
          style={{ padding: "2px 8px", fontSize: 11 }}
          onClick={() => setOpen((value) => !value)}
        >
          {open ? "▾" : "▸"} {profile.label || profile.id}
        </button>
        <code style={{ fontSize: 10, opacity: 0.6 }}>{profile.id}</code>
        {isDefault && (
          <span style={{ fontSize: 10, opacity: 0.75 }}>
            {t("codex.localAccess.toolRouterDefaultBadge", "兜底档案")}
          </span>
        )}
        <span style={{ fontSize: 10, opacity: 0.6 }}>
          {profile.allowNamespaces.length === 0
            ? t("codex.localAccess.toolRouterNoTrim", "不裁剪(全放行)")
            : `${profile.allowNamespaces.length} 项白名单`}
        </span>
      </div>

      {open && (
        <div style={{ display: "grid", gap: 6, marginTop: 8 }}>
          <label style={{ fontSize: 11, display: "grid", gap: 2 }}>
            {t("codex.localAccess.toolRouterAllow", "白名单(空 = 全部放行)")}
            <input
              type="text"
              value={profile.allowNamespaces.join(", ")}
              disabled={disabled}
              placeholder="ida, ghidra, frida"
              style={{ fontSize: 11 }}
              onChange={(event) =>
                onChange({
                  ...profile,
                  allowNamespaces: splitList(event.target.value),
                })
              }
            />
          </label>
          <label style={{ fontSize: 11, display: "grid", gap: 2 }}>
            {t("codex.localAccess.toolRouterDeny", "黑名单(优先于白名单)")}
            <input
              type="text"
              value={profile.denyNamespaces.join(", ")}
              disabled={disabled}
              placeholder="crawl, scrap"
              style={{ fontSize: 11 }}
              onChange={(event) =>
                onChange({
                  ...profile,
                  denyNamespaces: splitList(event.target.value),
                })
              }
            />
          </label>
          <label style={{ fontSize: 11, display: "grid", gap: 2 }}>
            {t("codex.localAccess.toolRouterProfileModel", "该类型使用的模型(留空 = 不改)")}
            <input
              type="text"
              value={profile.model}
              disabled={disabled}
              placeholder="gpt-5.6-sol"
              style={{ fontSize: 11 }}
              onChange={(event) => onChange({ ...profile, model: event.target.value })}
            />
          </label>
          <div style={{ fontSize: 10, opacity: 0.65, lineHeight: 1.5 }}>
            {t(
              "codex.localAccess.toolRouterProfileNote",
              "匹配规则: 命名空间名(如 mcp__codex_apps__gmail)包含任一关键词即命中。原生工具不受影响。",
            )}
          </div>
        </div>
      )}
    </div>
  );
}

/** 把逗号分隔的输入切成列表, 去空去重。 */
function splitList(raw: string): string[] {
  const seen = new Set<string>();
  const out: string[] = [];
  for (const part of raw.split(/[,，\n]/)) {
    const token = part.trim();
    if (token === "" || seen.has(token)) continue;
    seen.add(token);
    out.push(token);
  }
  return out;
}
