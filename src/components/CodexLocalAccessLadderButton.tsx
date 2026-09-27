import { useCallback, useEffect, useRef, useState } from "react";
import { Layers } from "lucide-react";
import { useTranslation } from "react-i18next";
import * as codexLocalAccessService from "../services/codexLocalAccessService";
import type { CodexLocalAccessLadderState } from "../types/codexLocalAccess";

/**
 * 破甲阶梯(五层降级链)控制入口。
 *
 * 与 cyber-shield 的关系:
 * - shield 是静态混淆档位(0-2), 只决定"怎么改写"。
 * - ladder 是"被拒就升级并重发"的降级链(0-4), 决定"被拒后用什么强度重试"。
 *
 * 两者共用 sidecar 的改写素材, 但状态独立存放, 互不覆盖。
 * 这里写入的 ladder-state.json 由 sidecar 在 3 秒内热加载, 不需要重启服务。
 */

/** 每一层的名称与适用场景, 用于面板上的说明文案。 */
const LADDER_LEVELS: Array<{ level: number; name: string; detail: string }> = [
  { level: 0, name: "直通", detail: "原文直接转发, 保真最高, 起始层" },
  { level: 1, name: "注入指令", detail: "追加执行纪律与交付契约锚点" },
  { level: 2, name: "关键词改写", detail: "按热加载词表替换术语, 动词与产物保留" },
  { level: 3, name: "中性化重写", detail: "把意图改写成技术规格语言" },
  { level: 4, name: "目标抽象化", detail: "URL/IP/域名换 IANA 保留值, 产出后回填真值(仅非流式)" },
];

export function CodexLocalAccessLadderButton({ disabled = false }: { disabled?: boolean }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [state, setState] = useState<CodexLocalAccessLadderState | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await codexLocalAccessService.getCodexLocalAccessLadderState();
      setState(next);
    } catch (err) {
      // sidecar 未启动时读取失败属正常: 状态文件仍会被下次写入创建。
      setError(String(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // 点击面板外部收起。用 mousedown 而不是 click, 避免与内部按钮的 click 抢事件。
  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: MouseEvent) => {
      if (!containerRef.current?.contains(event.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", onPointerDown);
    return () => document.removeEventListener("mousedown", onPointerDown);
  }, [open]);

  const applyLevel = useCallback(
    async (level: number) => {
      setBusy(true);
      setError(null);
      try {
        const next = await codexLocalAccessService.updateCodexLocalAccessLadderLevel(level);
        setState(next);
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  const applyConfig = useCallback(
    async (enabled: boolean, autoRetry: boolean, maxRetries: number) => {
      setBusy(true);
      setError(null);
      try {
        const next = await codexLocalAccessService.updateCodexLocalAccessLadder(
          enabled,
          autoRetry,
          maxRetries,
        );
        setState(next);
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [],
  );

  const enabled = state?.enabled ?? true;
  const level = state?.level ?? 0;
  const levelInfo = LADDER_LEVELS.find((item) => item.level === level) ?? LADDER_LEVELS[0];

  return (
    <div className="codex-local-access-ladder" ref={containerRef} style={{ position: "relative" }}>
      <button
        type="button"
        className={`folder-icon-btn codex-local-access-toolbar-btn${
          enabled ? " is-active" : ""
        }`}
        onClick={() => setOpen((value) => !value)}
        disabled={disabled}
        title={
          enabled
            ? `破甲阶梯已开启 — 当前第 ${level} 层(${levelInfo.name})。点击调整`
            : "破甲阶梯已关闭 — 点击开启"
        }
        aria-label="ladder toggle"
        aria-pressed={enabled}
        aria-expanded={open}
      >
        <Layers size={14} />
        {enabled && (
          <span style={{ fontSize: 9, fontWeight: 700, marginLeft: 2 }}>L{level}</span>
        )}
      </button>

      {open && (
        <div
          className="codex-local-access-ladder-panel"
          style={{
            position: "absolute",
            zIndex: 40,
            right: 0,
            top: "calc(100% + 6px)",
            width: 360,
            padding: 12,
            borderRadius: 10,
            background: "var(--surface-2, #1f2126)",
            border: "1px solid var(--border, #33373f)",
            boxShadow: "0 12px 32px rgba(0,0,0,0.35)",
            textAlign: "left",
          }}
        >
          <div
            style={{
              display: "flex",
              alignItems: "center",
              justifyContent: "space-between",
              gap: 8,
              marginBottom: 8,
            }}
          >
            <strong style={{ fontSize: 12 }}>
              {t("codex.localAccess.ladderTitle", "破甲阶梯")}
            </strong>
            <button
              type="button"
              className={`btn ${enabled ? "btn-primary" : "btn-secondary"}`}
              style={{ padding: "2px 8px", fontSize: 11 }}
              disabled={busy}
              onClick={() => void applyConfig(!enabled, state?.autoRetry ?? true, state?.maxRetries ?? 4)}
            >
              {enabled ? t("common.enabled", "已开启") : t("common.disabled", "已关闭")}
            </button>
          </div>

          <div style={{ fontSize: 11, opacity: 0.75, marginBottom: 8 }}>
            {t(
              "codex.localAccess.ladderHint",
              "被上游拒绝时自动升级到下一层并重发。流式只升级并提示重发, 非流式静默重试。到顶即如实报错。",
            )}
          </div>

          <div style={{ fontSize: 10.5, opacity: 0.62, marginBottom: 8 }}>
            {t(
              "codex.localAccess.ladderStreamCapHint",
              "流式请求封顶在 L3: L4 的占位符需要拿到完整产出后回填真值, 流式边收边发没有这个时机, 强行用 L4 会让你看到 example.com 而不是真实目标。",
            )}
          </div>

          <div style={{ display: "grid", gap: 4 }}>
            {LADDER_LEVELS.map((item) => {
              const active = item.level === level;
              return (
                <button
                  key={item.level}
                  type="button"
                  className={`btn ${active ? "btn-primary" : "btn-secondary"}`}
                  style={{
                    display: "block",
                    textAlign: "left",
                    padding: "6px 8px",
                    fontSize: 11,
                    lineHeight: 1.35,
                  }}
                  disabled={busy}
                  onClick={() => void applyLevel(item.level)}
                  title={item.detail}
                >
                  <span style={{ fontWeight: 700 }}>
                    L{item.level} {item.name}
                  </span>
                  <span style={{ display: "block", opacity: 0.72 }}>{item.detail}</span>
                </button>
              );
            })}
          </div>

          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              marginTop: 8,
              fontSize: 11,
            }}
          >
            <input
              type="checkbox"
              checked={state?.autoRetry ?? true}
              disabled={busy}
              onChange={(event) =>
                void applyConfig(
                  enabled,
                  event.target.checked,
                  state?.maxRetries ?? 4,
                )
              }
            />
            {t("codex.localAccess.ladderAutoRetry", "非流式被拒后自动重试")}
          </label>

          {error && (
            <div style={{ marginTop: 8, fontSize: 11, color: "var(--danger, #f0616d)" }}>
              {error}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
