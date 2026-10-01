import { useCallback, useEffect, useMemo, useState } from "react";
import {
  Check,
  ChevronDown,
  CircleAlert,
  CircleHelp,
  Power,
  RefreshCw,
  Shield,
  SlidersHorizontal,
  Wrench,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import * as codexService from "../services/codexService";
import * as localAccessService from "../services/codexLocalAccessService";
import type { CodexAccount } from "../types/codex";
import type {
  CodexLocalAccessLadderState,
  CodexLocalAccessState,
  CodexLocalAccessToolRouterState,
} from "../types/codexLocalAccess";
import { isCodexWebSessionAccount } from "../types/codex";
import "./CodexEnginePage.css";

function errorText(error: unknown): string {
  return String(error).replace(/^Error:\s*/, "").trim() || "操作失败";
}

function accountLabel(account: CodexAccount): string {
  return account.email?.trim() || account.account_name?.trim() || account.id;
}

export function CodexEnginePage() {
  const { t } = useTranslation();
  const [state, setState] = useState<CodexLocalAccessState | null>(null);
  const [accounts, setAccounts] = useState<CodexAccount[]>([]);
  const [ladder, setLadder] = useState<CodexLocalAccessLadderState | null>(null);
  const [toolRouter, setToolRouter] = useState<CodexLocalAccessToolRouterState | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");

  const availableAccounts = useMemo(
    () => accounts.filter((account) => !isCodexWebSessionAccount(account)),
    [accounts],
  );
  const selectedIds = state?.engineStandaloneAccountIds ?? [];
  const selectedSet = useMemo(() => new Set(selectedIds), [selectedIds]);

  const refresh = useCallback(async () => {
    setError("");
    try {
      const [nextState, nextAccounts, nextLadder, nextToolRouter] = await Promise.all([
        localAccessService.getCodexLocalAccessState(),
        codexService.listCodexAccounts(),
        localAccessService.getCodexLocalAccessLadderState(),
        localAccessService.getCodexLocalAccessToolRouterState(),
      ]);
      setState(nextState);
      setAccounts(nextAccounts);
      setLadder(nextLadder);
      setToolRouter(nextToolRouter);
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const update = async (action: () => Promise<CodexLocalAccessState>) => {
    setBusy(true);
    setError("");
    try {
      setState(await action());
    } catch (cause) {
      setError(errorText(cause));
    } finally {
      setBusy(false);
    }
  };

  const toggleEnabled = () => {
    const enabled = !(state?.engineStandaloneEnabled ?? false);
    const ids = selectedIds.length > 0 ? selectedIds : availableAccounts.map((account) => account.id);
    void update(() => localAccessService.setCodexEngineStandaloneEnabled(enabled, enabled ? ids : undefined));
  };

  const toggleAccount = (accountId: string) => {
    const next = selectedSet.has(accountId)
      ? selectedIds.filter((id) => id !== accountId)
      : [...selectedIds, accountId];
    void update(() => localAccessService.setCodexEngineStandaloneAccounts(next));
  };

  const restart = () => {
    void update(() => localAccessService.restartCodexLocalAccessSidecar());
  };

  const statusLabel = loading
    ? t("codex.engine.loading", "加载中")
    : state?.running
      ? t("codex.engine.running", "运行中")
      : state?.engineStandaloneEnabled
        ? t("codex.engine.starting", "已启用，等待启动")
        : t("codex.engine.stopped", "已停止");

  return (
    <main className="codex-engine-page">
      <header className="codex-engine-header">
        <div>
          <div className="codex-engine-kicker">
            <Shield size={16} aria-hidden="true" />
            {t("codex.engine.kicker", "CODEX ENGINE")}
          </div>
          <h1>{t("codex.engine.title", "破甲引擎")}</h1>
          <p>{t("codex.engine.subtitle", "独立运行的本地 sidecar 引擎，用于 Codex 请求调度与防护。")}</p>
        </div>
        <button
          type="button"
          className={`codex-engine-power ${state?.engineStandaloneEnabled ? "is-on" : ""}`}
          onClick={toggleEnabled}
          disabled={loading || busy}
          aria-pressed={state?.engineStandaloneEnabled ?? false}
        >
          <Power size={18} aria-hidden="true" />
          <span>{state?.engineStandaloneEnabled ? t("common.enabled", "已启用") : t("common.disabled", "已停用")}</span>
        </button>
      </header>

      {error ? (
        <div className="codex-engine-alert error" role="alert">
          <CircleAlert size={16} aria-hidden="true" />
          <span>{error}</span>
          <button type="button" onClick={() => void refresh()} disabled={busy}>
            {t("common.retry", "重试")}
          </button>
        </div>
      ) : null}

      <section className="codex-engine-grid">
        <div className="codex-engine-panel codex-engine-status-panel">
          <div className="codex-engine-panel-heading">
            <div>
              <span className="codex-engine-eyebrow">{t("codex.engine.status", "运行状态")}</span>
              <strong className={`codex-engine-status-dot ${state?.running ? "is-running" : ""}`}>
                <span /> {statusLabel}
              </strong>
            </div>
            <button type="button" className="codex-engine-icon-button" onClick={restart} disabled={!state?.engineStandaloneEnabled || busy} title={t("codex.engine.restart", "重启 sidecar")}>
              <RefreshCw size={16} aria-hidden="true" />
            </button>
          </div>
          <div className="codex-engine-metrics">
            <div>
              <span>{t("codex.engine.port", "本机端口")}</span>
              <code>{state?.collection?.port ?? "--"}</code>
            </div>
            <div>
              <span>{t("codex.engine.upstreams", "上游账号")}</span>
              <strong>{selectedIds.length}</strong>
            </div>
            <div>
              <span>{t("codex.engine.endpoint", "监听地址")}</span>
              <code>127.0.0.1</code>
            </div>
          </div>
        </div>

        <div className="codex-engine-panel codex-engine-account-panel">
          <div className="codex-engine-panel-heading">
            <div>
              <span className="codex-engine-eyebrow">{t("codex.engine.upstreamTitle", "上游账号")}</span>
              <h2>{t("codex.engine.upstreamHeading", "选择引擎使用的账号")}</h2>
            </div>
            <CircleHelp size={17} aria-label={t("codex.engine.upstreamHelp", "仅选中的账号会进入独立 sidecar 账号池")} />
          </div>
          <div className="codex-engine-account-list">
            {availableAccounts.length === 0 ? (
              <div className="codex-engine-empty">{t("codex.engine.noAccounts", "暂无可用 Codex 账号")}</div>
            ) : (
              availableAccounts.map((account) => {
                const checked = selectedSet.has(account.id);
                return (
                  <label className={`codex-engine-account ${checked ? "is-selected" : ""}`} key={account.id}>
                    <input type="checkbox" checked={checked} onChange={() => toggleAccount(account.id)} disabled={busy} />
                    <span className="codex-engine-check"><Check size={13} aria-hidden="true" /></span>
                    <span className="codex-engine-account-copy">
                      <strong>{accountLabel(account)}</strong>
                      <small>{account.plan_type?.trim() || t("codex.engine.accountDefaultPlan", "Codex account")}</small>
                    </span>
                    <span className="codex-engine-account-id">{account.id.slice(0, 8)}</span>
                  </label>
                );
              })
            )}
          </div>
          <p className="codex-engine-hint">{t("codex.engine.upstreamHint", "账号选择只影响独立引擎，不会修改原有账号资料或 API 服务配置。")}</p>
        </div>
      </section>

      <section className="codex-engine-controls">
        <div className="codex-engine-panel">
          <div className="codex-engine-panel-heading">
            <div className="codex-engine-control-title"><SlidersHorizontal size={17} aria-hidden="true" /><h2>{t("codex.engine.ladder", "破甲阶梯")}</h2></div>
            <label className="codex-engine-switch"><input type="checkbox" checked={ladder?.enabled ?? false} onChange={(event) => ladder && void localAccessService.updateCodexLocalAccessLadder(event.target.checked, ladder.autoRetry, ladder.maxRetries).then(setLadder).catch((cause) => setError(errorText(cause)))} disabled={!ladder || busy} /><span /></label>
          </div>
          <p>{t("codex.engine.ladderHint", "请求被上游拦截时，按档位逐步调整请求内容并重试。")}</p>
          <div className="codex-engine-range-row">
            <input type="range" min="0" max="4" value={ladder?.level ?? 0} onChange={(event) => void localAccessService.updateCodexLocalAccessLadderLevel(Number(event.target.value)).then(setLadder).catch((cause) => setError(errorText(cause)))} disabled={!ladder || busy} />
            <span>L{ladder?.level ?? 0}</span>
          </div>
        </div>

        <div className="codex-engine-panel">
          <div className="codex-engine-panel-heading">
            <div className="codex-engine-control-title"><Wrench size={17} aria-hidden="true" /><h2>{t("codex.engine.toolRouter", "工具路由")}</h2></div>
            <label className="codex-engine-switch"><input type="checkbox" checked={toolRouter?.enabled ?? false} onChange={(event) => void localAccessService.updateCodexLocalAccessToolRouterEnabled(event.target.checked).then(setToolRouter).catch((cause) => setError(errorText(cause)))} disabled={!toolRouter || busy} /><span /></label>
          </div>
          <p>{t("codex.engine.toolRouterHint", "根据请求内容选择工具策略，并在必要时裁剪不兼容工具。")}</p>
          <div className="codex-engine-control-meta"><span>{t("codex.engine.profile", "当前档案")}</span><code>{toolRouter?.defaultProfile || "default"}</code><ChevronDown size={15} aria-hidden="true" /></div>
        </div>
      </section>
    </main>
  );
}
