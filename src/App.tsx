import React, { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Layers,
  RotateCw,
  Plus,
  Trash2,
  CheckCircle2,
  Power,
  Zap,
  RefreshCcw,
  Globe,
  Loader2,
  DownloadCloud,
  Check,
  AlertCircle,
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
  consecutive_hot_reloads: number;
  max_safe_hot_reloads: number;
}

interface SwitchResult {
  mode: string;
  message: string;
}

function AntigravityLogo({ size = 22 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 1024 1024"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={{ display: "block" }}
    >
      <defs>
        <linearGradient id="uiAGrad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" stopColor="#ffffff" />
          <stop offset="25%" stopColor="#38bdf8" />
          <stop offset="70%" stopColor="#0284c7" />
          <stop offset="100%" stopColor="#1e40af" />
        </linearGradient>
        <linearGradient id="uiRingBack" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#0284c7" stopOpacity="0.4" />
          <stop offset="100%" stopColor="#38bdf8" stopOpacity="0.6" />
        </linearGradient>
        <linearGradient id="uiRingFront" x1="0%" y1="0%" x2="100%" y2="0%">
          <stop offset="0%" stopColor="#0284c7" />
          <stop offset="40%" stopColor="#38bdf8" />
          <stop offset="60%" stopColor="#ffffff" />
          <stop offset="100%" stopColor="#0284c7" />
        </linearGradient>
      </defs>
      {/* 45° 倾斜椭圆环（后半段） */}
      <g transform="translate(512, 530) rotate(-45)">
        <path
          d="M -410,0 A 410,140 0 0,1 410,0"
          stroke="url(#uiRingBack)"
          strokeWidth="60"
          strokeLinecap="round"
        />
      </g>
      {/* 字母 A 主体 */}
      <path
        d="M 480,210 L 544,210 L 740,750 L 630,750 L 582,610 L 442,610 L 394,750 L 284,750 Z M 464,530 L 560,530 L 512,380 Z"
        fill="url(#uiAGrad)"
        fillRule="evenodd"
      />
      {/* 45° 倾斜椭圆环（前半段包裹） */}
      <g transform="translate(512, 530) rotate(-45)">
        <path
          d="M 410,0 A 410,140 0 0,1 -410,0"
          stroke="url(#uiRingFront)"
          strokeWidth="64"
          strokeLinecap="round"
        />
        <circle cx="120" cy="136" r="22" fill="#ffffff" />
      </g>
    </svg>
  );
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
  const [hotReload, setHotReload] = useState(true);

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
      setSuccessMsg(`🎉 成功添加新账号：${account.email} (${account.name})`);
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

  // 3. 一键切换账号（支持极速无感热切 + 看门狗防熔断降级保护）
  const handleSwitch = async (id: string) => {
    setSwitchingId(id);
    setErrorMsg(null);
    setSuccessMsg(null);
    try {
      const result = await invoke<SwitchResult>("switch_account", { id, hotReload });
      await fetchState();
      setSuccessMsg(result.message);
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

  // 5. 核心热重载
  const handleHotReload = async () => {
    setLoading(true);
    setErrorMsg(null);
    setSuccessMsg(null);
    try {
      await invoke("hot_reload_antigravity");
      await fetchState();
      setSuccessMsg("⚡ 语言服务器核心大脑已重载复活！");
    } catch (err: any) {
      setErrorMsg("热重载失败: " + String(err));
    } finally {
      setLoading(false);
    }
  };

  // 6. 重启桌面端 (冷重启)
  const handleRestart = async () => {
    setLoading(true);
    try {
      await invoke("restart_antigravity");
      await fetchState();
      setSuccessMsg("桌面端完整重启完成");
    } catch (err: any) {
      setErrorMsg("重启失败: " + String(err));
    } finally {
      setLoading(false);
    }
  };

  // 7. 终止桌面端
  const handleKill = async () => {
    try {
      await invoke("kill_antigravity");
      await fetchState();
      setSuccessMsg("反重力客户端所有进程已安全退出");
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
            <AntigravityLogo size={24} />
          </div>
          <div>
            <div className="brand-title">Antigravity 账号助手</div>
            <div className="brand-subtitle">秒级无感热切 · 托盘常驻</div>
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

      {/* 消息提示栏 */}
      {errorMsg && (
        <div className="msg-box error">
          <AlertCircle size={14} />
          <span>{errorMsg}</span>
        </div>
      )}
      {successMsg && (
        <div className="msg-box success">
          <Check size={14} />
          <span>{successMsg}</span>
        </div>
      )}

      {/* 当前桌面端激活的账号 */}
      <div className="active-banner">
        <div className="active-banner-header">
          <span>当前客户端会话凭据</span>
          {state?.current_email && <span className="status-badge">有效授权</span>}
        </div>
        <div className="active-email">
          {state?.current_email || "（未检测到已登录的 Google 凭据）"}
        </div>
        <div className="active-meta">
          <span>Scope: {state?.current_user_scope || "默认"}</span>
        </div>
      </div>

      {/* OAuth 独立登录区域 */}
      <div className="oauth-cta-card">
        {isAuthorizing ? (
          <div className="authorizing-box">
            <Loader2 size={16} className="spin" />
            <span>已在默认浏览器打开 Google 授权页面，请在网页中完成授权...</span>
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
                placeholder="账号别名（选填，如：备用号 / Team-2）"
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

      {/* 极速热切换控制栏 */}
      <div className="mode-toggle-bar">
        <label className="toggle-label" title="开启后仅热重载后台核心大脑，Electron 主窗口完全不关闭；连续切号达上限时会自动改用安全冷重启防熔断">
          <input
            type="checkbox"
            checked={hotReload}
            onChange={(e) => setHotReload(e.target.checked)}
          />
          <span className="toggle-text">
            <Zap size={13} className="text-accent" />
            <b>极速无感热切</b> (窗口不关，约2秒重连)
          </span>
        </label>
        <span className="mode-desc">
          {!hotReload
            ? "🔄 完整冷重启客户端"
            : state && state.consecutive_hot_reloads >= state.max_safe_hot_reloads
            ? "🛡️ 下次自动防熔断冷重启"
            : state && state.consecutive_hot_reloads > 0
            ? `⚡ 连续热切 ${state.consecutive_hot_reloads}/${state.max_safe_hot_reloads}`
            : "⚡ 仅重载后台 LS"}
        </span>
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
                  className={`btn-switch ${hotReload ? "is-hot" : ""}`}
                  disabled={acc.is_active || switchingId === acc.id}
                  onClick={() => handleSwitch(acc.id)}
                  title={hotReload ? "极速热切：不关闭主窗口" : "普通切换：重启桌面端"}
                >
                  <RotateCw
                    size={13}
                    className={switchingId === acc.id ? "spin" : ""}
                  />
                  <span>
                    {acc.is_active
                      ? "使用中"
                      : hotReload
                      ? "⚡ 极速热切"
                      : "一键切换"}
                  </span>
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
          <button
            className="btn-ghost"
            onClick={handleHotReload}
            disabled={loading}
            title="仅重启后台语言服务器核心大脑，主窗口不关闭"
          >
            <Zap size={12} className="text-accent" />
            <span>热重载核心</span>
          </button>
          <button
            className="btn-ghost"
            onClick={handleRestart}
            disabled={loading}
            title="完整重启反重力桌面端（冷重启）"
          >
            <RefreshCcw size={12} />
            <span>冷重启</span>
          </button>
          <button
            className="btn-ghost"
            onClick={handleKill}
            title="完全终止反重力所有进程"
          >
            <Power size={12} />
            <span>终止</span>
          </button>
        </div>

        <div style={{ fontSize: 10, opacity: 0.7 }}>
          <span>点击右上角 ✖ 自动最小化至托盘</span>
        </div>
      </footer>
    </div>
  );
}
