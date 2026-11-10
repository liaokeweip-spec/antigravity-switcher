# Antigravity Switcher (反重力账号助手)

专为 **Google Antigravity 桌面端（Antigravity 2.0 Desktop Electron）** 打造的高性能、零风控、原生多账号一键秒切工具。

---

## 核心特性

- ⚡ **毫秒级原子切换**：一键置换 Windows 凭据管理器、用户作用域（Scope ID）与环境状态，平滑重启桌面端，耗时 < 1 秒。
- 🛡️ **绝对零封号风险**：坚决摒弃第三方工具高危的“本地 API 代理池（Relay）”模式。所有账号均通过官方桌面端正常 OAuth 授权，纯本地凭据搬运，在 Google 风控视角下完全等同于正常人工切换账号。
- 🔐 **DPAPI 本地硬件级加密**：账号池数据使用 Windows 原生 `CryptProtectData`（数据保护 API）加密，只有当前机器与当前登录用户才能解密，杜绝凭据泄露。
- 🎯 **系统托盘常驻（System Tray）**：支持常驻托盘与全局快捷操作，右键即可直接切换目标账号，无需频繁开启主界面。
- 📦 **超轻量工业级架构**：基于 **Tauri v2 + Rust + React 19 + TypeScript** 构建，内存占用仅约 20MB。

---

## 底层技术机理

不同于 VS Code / IDE 插件修改 `state.vscdb` (SQLite)，反重力桌面端是一套独立的 Electron 宿主与 Go/C++ 语言服务（`language_server.exe`）架构。本工具通过 Rust 原生系统调用精准打通了四重数据链路：

```
[ Antigravity Switcher ]
        │
        ├─ 1. Windows Credential Manager: 读写 "gemini:antigravity" (OAuth JSON)
        ├─ 2. 作用域同步: %APPDATA%/Antigravity/credential-user-scope
        ├─ 3. 全局状态: ~/.gemini/google_accounts.json
        └─ 4. 进程调度: 优雅终止与拉起 Antigravity.exe & language_server.exe
```

---

## 使用指南

### 1. 录入你的第一个账号（如：主力号）
1. 在反重力桌面端正常登录账号 A。
2. 打开 **Antigravity 账号助手**，在输入框填写别名（如 `主力号`）。
3. 点击 **「保存当前账号」**，当前登录会话即刻加密归档入库。

### 2. 录入你的第二个账号（如：备用号）
1. 在反重力桌面端点击头像退出登录，登录账号 B。
2. 在助手输入框填写别名（如 `备用号`），点击 **「保存当前账号」**。

### 3. 一键切换
- 在列表中直接点击对应账号右侧的 **「一键切换」** 按钮；
- 或在右下角系统托盘图标右键快速选择目标账号。
- 工具将自动接管凭据置换并重启桌面端，直接进入新账号状态！

---

## 开发与构建

```bash
# 安装前端依赖
pnpm install

# 本地热重载开发
pnpm tauri dev

# 打包发布生产免安装单文件 EXE
pnpm tauri build
```
