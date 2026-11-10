import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Layers,
  RotateCw,
  Plus,
  Trash2,
  CheckCircle2,
  Power,
  ShieldCheck,
  Zap,
  RefreshCcw,
  Globe,
  Loader2,
  DownloadCloud,
} from "lucide-react";
import "./App.css";

interface AccountItem {
  id: string;
  name: string;
  email: string;
  user_scope: string;
  is_active: boolean;
  created_at: string;
}

interface AppState {
  current_email: string | null;
  current_user_scope: string | null;
  is_antigravity_running: boolean;
  accounts: AccountItem[];
}

export default function App() {
  const [state, setState] = useState<AppState | null>(null);
  const [loading, setLoading] = useState(false);
  const [isAuthorizing, setIsAuthorizing] = useState(false);
  const [switchingId, setSwitchingId] = useState<string | null>(null);
  const [aliasInput, setAliasInput] = useState("");
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [successMsg, setSuccessMsg] = useState<string | null>(null);
  const [showManualSnapshot, setShowManualSnapshot] = useState(false);

  const fetchState = async () => {
    try {
      const res = await invoke<AppState>("get_state");
      setState(res);
      setErrorMsg(null);
    } catch (err: any) {
      setErrorMsg(String(err));
    }
  };

  useEffect(() => {
    fetchState();
    const interval = setInterval(fetchState, 3000);
    return () => clearInterval(interval);
  }, []);

  // 1. 软件自己发起 Google OAuth 登录新账号
  const handleStartOAuthLogin = async () => {
    setIsAuthorizing(true);
    setErrorMsg(null);
    setSuccessMsg(null);
    try {
      const alias = aliasInput.trim() || undefined;
      const account = await invoke<AccountItem>("start_google_login", { alias });
      setSuccessMsg(`✅ 成功添加新账号：${account.email} (${account.name})`);
      setAliasInput("");
      await fetchState();
    } catch (err: any) {
      setErrorMsg("登录授权未完成: " + String(err));
    } finally {
      setIsAuthorizing(false);
    }
  };

  // 2. 备用方式：从当前桌面端抓取会话
  const handleSnapshotCurrent = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!aliasInput.trim()) return;
    setLoading(true);
    try {
      await invoke("snapshot_current", { name: aliasInput.trim() });
      setAliasInput("");
      setSuccessMsg("已保存当前桌面端会话");
      await fetchState();
    } catch (err: any) {
      setErrorMsg("保存失败: " + String(err));
    } finally {
      setLoading(false);
    }
  };

  // 3. 一键切换账号
  const handleSwitch = async (id: string) => {
    setSwitchingId(id);
    setErrorMsg(null);
    try {
      await invoke("switch_account", { id });
      await fetchState();
    } catch (err: any) {
      setErrorMsg("切换失败: " + String(err));
    } finally {
      setSwitchingId(null);
    }
  };

  // 4. 删除账号
  const handleDelete = async (id: string, name: string) => {
    if (!confirm(`确定要从本地账号池中删除【${name}】吗？`)) return;
    try {
      await invoke("delete_account", { id });
      await fetchState();
    } catch (err: any) {
      setErrorMsg("删除失败: " + String(err));
    }
  };

  // 5. 重启桌面端
  const handleRestart = async () => {
    setLoading(true);
    try {
      await invoke("restart_antigravity");
      await fetchState();
    } catch (err: any) {
      setErrorMsg("重启失败: " + String(err));
    } finally {
      setLoading(false);
    }
  };

  // 6. 终止桌面端
  const handleKill = async () => {
    try {
      await invoke("kill_antigravity");
      await fetchState();
    } catch (err: any) {
      setErrorMsg("停止失败: " + String(err));
    }
  };

  return (
    <div className="app-container">
      {/* 顶部标题栏 */}
      <header className="app-header">
        <div className="brand-wrapper">
          <div className="brand-icon">
            <Zap size={18} />
          </div>
          <div>
            <div className="brand-title">Antigravity 账号助手</div>
            <div className="brand-subtitle">桌面端一键切换 · 内置独立授权</div>
          </div>
        </div>

        <div className="header-status">
          <span
            className={`status-dot ${
              state?.is_antigravity_running ? "online" : "offline"
            }`}
          />
          <span>
            {state?.is_antigravity_running ? "反重力运行中" : "反重力已停止"}
          </span>
        </div>
      </header>

      {/* 错误提示横幅 */}
      {errorMsg && (
        <div style={{ background: "rgba(239, 68, 68, 0.2)", border: "1px solid rgba(239, 68, 68, 0.4)", borderRadius: 6, padding: "6px 10px", fontSize: 11, color: "#fca5a5" }}>
          {errorMsg}
        </div>
      )}

      {/* 成功提示横幅 */}
      {successMsg && (
        <div style={{ background: "rgba(16, 185, 129, 0.2)", border: "1px solid rgba(16, 185, 129, 0.4)", borderRadius: 6, padding: "6px 10px", fontSize: 11, color: "#6ee7b7" }}>
          {successMsg}
        </div>
      )}

      {/* 当前桌面端激活状态横幅 */}
      <div className="active-banner">
        <div className="active-banner-header">
          <span>当前客户端会话 (Active Session)</span>
          <ShieldCheck size={14} />
        </div>
        <div className="active-email">
          {state?.current_email || "未检测到有效登录凭据"}
        </div>
        <div className="active-meta">
          <span>
            Scope ID: {state?.current_user_scope ? state.current_user_scope.substring(0, 10) + "..." : "未知"}
          </span>
          <span>·</span>
          <span>DPAPI 本地密保</span>
        </div>
      </div>

      {/* 核心登录与添加卡片 */}
      <div className="oauth-cta-card">
        {isAuthorizing ? (
          <div className="authorizing-box">
            <Loader2 size={16} className="spin" />
            <span>已唤起浏览器，请在 Google 页面确认授权...</span>
          </div>
        ) : (
          <>
            <button
              className="btn-oauth-main"
              onClick={handleStartOAuthLogin}
              disabled={loading || isAuthorizing}
            >
              <Globe size={16} />
              <span>➕ 登录并添加新 Google 账号</span>
            </button>

            <div className="oauth-input-row">
              <input
                type="text"
                className="oauth-input"
                placeholder="账号别名（选填，如：二号备用号）"
                value={aliasInput}
                onChange={(e) => setAliasInput(e.target.value)}
                disabled={isAuthorizing}
              />
              <button
                type="button"
                className="btn-secondary"
                onClick={() => setShowManualSnapshot(!showManualSnapshot)}
                title="折叠/展开桌面端抓取"
              >
                <DownloadCloud size={12} />
                <span>抓取桌面端</span>
              </button>
            </div>

            {showManualSnapshot && (
              <div style={{ marginTop: 4, display: "flex", gap: 6 }}>
                <button
                  type="button"
                  className="btn-secondary"
                  style={{ flex: 1, justifyContent: "center" }}
                  onClick={handleSnapshotCurrent}
                  disabled={!aliasInput.trim() || !state?.current_email}
                >
                  <Plus size={12} />
                  <span>保存当前桌面端已登账号为【{aliasInput || "未命名"}】</span>
                </button>
              </div>
            )}
          </>
        )}
      </div>

      {/* 账号池列表 */}
      <div className="accounts-container">
        {state?.accounts && state.accounts.length > 0 ? (
          state.accounts.map((acc) => (
            <div
              key={acc.id}
              className={`account-card ${acc.is_active ? "is-active" : ""}`}
            >
              <div className="account-info">
                <div className="account-title-row">
                  <span className="account-name">{acc.name}</span>
                  {acc.is_active && (
                    <span className="badge-active">
                      <CheckCircle2 size={10} style={{ display: "inline", marginRight: 2 }} />
                      当前使用中
                    </span>
                  )}
                </div>
                <div className="account-email" title={acc.email}>
                  {acc.email}
                </div>
              </div>

              <div className="account-actions">
                <button
                  className="btn-switch"
                  disabled={acc.is_active || switchingId === acc.id}
                  onClick={() => handleSwitch(acc.id)}
                >
                  <RotateCw
                    size={13}
                    className={switchingId === acc.id ? "spin" : ""}
                  />
                  <span>{acc.is_active ? "使用中" : "一键切换"}</span>
                </button>

                <button
                  className="btn-icon-danger"
                  title="删除此账号"
                  onClick={() => handleDelete(acc.id, acc.name)}
                >
                  <Trash2 size={14} />
                </button>
              </div>
            </div>
          ))
        ) : (
          <div className="empty-state">
            <Layers size={36} opacity={0.3} />
            <p>本地账号池暂无归档</p>
            <p style={{ fontSize: 11, maxWidth: 300 }}>
              直接点击上方的 <b>【➕ 登录并添加新 Google 账号】</b> 按钮，即可在浏览器中添加任意账号，全程不干扰桌面端！
            </p>
          </div>
        )}
      </div>

      {/* 底部控制与辅助操作 */}
      <footer className="app-footer">
        <div style={{ display: "flex", gap: 6 }}>
          <button className="btn-ghost" onClick={handleRestart} disabled={loading} title="重启反重力桌面端">
            <RefreshCcw size={12} />
            <span>重启桌面端</span>
          </button>
          <button className="btn-ghost" onClick={handleKill} title="完全终止反重力进程">
            <Power size={12} />
            <span>终止桌面端</span>
          </button>
        </div>

        <div>
          <span>独立 Loopback 授权 · 秒级热切</span>
        </div>
      </footer>
    </div>
  );
}
