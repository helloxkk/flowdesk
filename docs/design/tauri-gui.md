# FlowDesk Tauri GUI 设计文档

**状态:** Approved(用户已确认核心决策,Phase 0 可启动)
**日期:** 2026-06-23
**作者:** helloxkk(基于代码探路产出)
**范围:** macOS,服务端模式优先
**前端:** React
**设置存储:** JSON
**Bundle ID:** `com.flowdesk.app`

---

## 1. 背景与动机

FlowDesk(Barrier fork)的现有 GUI 是 Qt5 实现(`src/gui/`)。本设计文档描述将其替换为
**Tauri(Rust 后端 + Web 前端)** GUI 的方案。

**动机:**
- Qt5 构建依赖重、安装体验差(已在构建流程中暴露)
- 期望更现代的 UI 审美与交互
- Rust 后端与 C++ 核心天然亲近,适合进程管理场景
- Tauri 应用体积小(数 MB),适合系统工具

**不在本设计范围内:**
- Windows 服务模式(barrierd + IPC)—— 后置为独立项目
- Linux 支持 —— 后续 Phase 扩展
- 客户端模式(Phase 2 之后)

---

## 2. 架构总览

### 2.1 进程模型(关键决策)

**核心结论:macOS 上完全不用 IPC 协议。**

现有 Qt GUI 在非 Windows 平台采用 "Desktop 模式":
GUI 直接 `QProcess` 启动 `barriers -f`,通过 **stdout 扫描状态**、**stdin 写 1 字节** `'S'` 停止。
IPC 协议(`127.0.0.1:24801`)仅在 Windows 服务模式下使用,macOS 不涉及。

**新 Tauri GUI 复用完全相同的模型:**

```
┌──────────────────────────────────────────────────────────┐
│  FlowDesk.app(Tauri)                                      │
│  ┌────────────────────┐    ┌──────────────────────────┐  │
│  │  WebView(前端)    │◄──►│  Rust 后端                │  │
│  │  React/Svelte UI   │    │  - 进程监督子模块        │  │
│  │                    │    │  - 配置文件生成器        │  │
│  └────────────────────┘    │  - 日志解析器            │  │
│                            │  - 设置持久化            │  │
│                            └──────────┬───────────────┘  │
└───────────────────────────────────────┼──────────────────┘
                                        │ spawn + pipes
                                        ▼
                              ┌──────────────────────┐
                              │  barriers -f -c cfg  │  ← C++ 核心,不动
                              │  (现有二进制)        │
                              └──────────┬───────────┘
                                         │ TCP 24800
                                         ▼
                              ┌──────────────────────┐
                              │  barrierc(其它机器) │
                              └──────────────────────┘
```

Tauri 与 `barriers` 之间是**父子进程 + 管道**关系,不引入新的网络协议。
C++ 核心代码**一行不改**。

### 2.2 仓库目录结构

新 GUI 代码放在 `src/gui-tauri/`,与现有 `src/gui/` 共存,便于对照开发与渐进替换:

```
flowdesk/
  src/
    gui/             # 旧 Qt5 GUI(保留,最终删除)
    gui-tauri/       # 新 Tauri GUI ★ 新增
      src-tauri/     # Rust 后端
        Cargo.toml
        src/
          main.rs
          supervisor.rs    # 进程监督子模块
          config.rs        # barrier 文本配置生成
          logparse.rs      # stdout 状态/指纹解析
          settings.rs      # 应用设置持久化(JSON)
          commands.rs      # Tauri command(invokable from JS)
        icons/
        tauri.conf.json    # bundle id: com.flowdesk.app
        entitlements.plist
      src/            # 前端(React + TypeScript + Vite)
      package.json
      vite.config.ts
  build/              # C++ 构建产物(不变)
  dist/macos/         # C++ 打包(不变)
```

CMake 主工程**不改动**,新 GUI 是独立的 npm/cargo 工程,通过引用 `build/bin/barriers`
路径启动核心二进制。

### 2.3 构建与分发

- **C++ 核心:** 仍走 CMake,产出 `build/bin/barriers`(server 二进制)
- **Tauri GUI:** 独立 `cargo build` / `npm run tauri build`,打出 `.app`
- **最终分发:** `.app` 内打包 `barriers` 二进制(类似现有 Barrier.app 结构)

---

## 3. 详细设计

### 3.1 进程监督子模块(supervisor.rs)

**职责:** 启动/停止/监督 `barriers` 子进程,把状态变化上报给前端。

**启动参数构造(参照现有 `MainWindow::serverArgs`):**

```
barriers -f --no-tray --debug <level> --name <screenName> \
         -c <configFile> --address [<listenIP>]:24800 \
         [--disable-crypto] [--enable-drag-drop]
```

| 字段 | 来源 | 默认 |
|------|------|------|
| `<level>` | Settings.logLevel | `INFO` |
| `<screenName>` | Settings.screenName | 主机名 |
| `<configFile>` | 临时文件(Settings 或屏幕编辑器生成) | — |
| `<listenIP>` | Settings.interface | 空串(监听全部) |
| `--disable-crypto` | Settings.cryptoEnabled == false | TLS 默认开 |
| `--enable-drag-drop` | Settings.enableDragAndDrop | true(非 Linux) |

**状态机(对齐现有 `qBarrierState` 枚举):**

```
       start()                检测到 "started server"
Stopped ───────► Starting ──────────────────────────► Connected
   ▲                │                                        │
   │                │ spawn 失败 / exit != 0                 │ 检测到退出
   │                ▼                                        ▼
   └─────────── Error                                   Disconnected
                       │                              │
                       └──────── auto-restart 1s ─────┘
```

**日志扫描规则(对照 `MainWindow::checkConnected`):**

| stdout 子串 | 状态转换 |
|-------------|---------|
| `started server` | → Connected |
| `connected to server` | → Connected(客户端模式才用) |
| `server status: active` | → Connected(IPC/服务模式,这里不用) |
| `cannot listen for clients` | → Error(端口占用) |
| `failed to connect to server` | → Error |
| 进程退出码 3(kExitArgs) | → Error(参数错) |
| 进程退出码 4(kExitConfig) | → Error(配置错) |
| 进程退出码 1(kExitFailed) | → Error(运行失败) |

**停止流程:** 向 stdin 写入单字节 `b'S'`,等最多 5 秒;超时则 `kill`(SIGTERM → SIGKILL)。

**自动重启:** 若期望状态为 Started 且进程退出,1 秒后重启(对齐现有行为)。

### 3.2 配置文件生成器(config.rs)

生成 barrier 文本配置(供 `barriers -c` 消费)。最小示例:

```
section: screens
    <serverScreenName>:
    <clientScreenName>:
end

section: links
    <serverScreenName>:
        right = <clientScreenName>
    <clientScreenName>:
        left = <serverScreenName>
end

section: options
end
```

**支持的字段(Phase 2 起逐步完善):**
- Phase 2: 仅 `screens` + `links`(矩形布局)
- Phase 3: 加 `aliases`、`switchCorners`、`switchCornerSize`、modifiers、fixes
- Phase 4: 加 hotkey(input filter 规则)

**数据结构(serde 序列化到应用设置):**

```rust
struct ServerConfig {
    num_columns: u32,      // 默认 5
    num_rows: u32,         // 默认 3
    screens: Vec<Screen>,  // 长度恒为 num_columns * num_rows
    has_heartbeat: bool,
    heartbeat: u32,
    relative_mouse_moves: bool,
    screen_saver_sync: bool,
    win32_keep_foreground: bool,
    has_switch_delay: bool,
    switch_delay: u32,
    has_switch_double_tap: bool,
    switch_double_tap: u32,
    switch_corner_size: u32,
    switch_corners: [bool; 4],   // TopLeft/TopRight/BottomLeft/BottomRight
    ignore_auto_config_client: bool,
    enable_drag_and_drop: bool,
    clipboard_sharing: bool,
    hotkeys: Vec<Hotkey>,
}

struct Screen {
    name: String,
    aliases: Vec<String>,
    modifiers: [Option<String>; 5],   // shift/ctrl/alt/meta/super
    switch_corners: [bool; 4],
    switch_corner_size: u32,
    fixes: Fixes,                       // caps/num/scroll lock, xtest, preserveFocus
}

struct Hotkey {
    keys: KeySequence,
    actions: Vec<Action>,
}
```

屏幕邻接关系**由网格位置隐式推导**(不是每屏存储),与现有实现一致。

### 3.3 日志解析器(logparse.rs)

逐行解析子进程 stdout,产出三类事件:

```rust
enum LogEvent {
    Line { timestamp: String, level: Level, message: String },
    StateChange(State),       // 由关键字触发
    FingerprintPrompt {       // TLS 信任提示
        sha1: String,
        sha256: String,
    },
}
```

**日志行格式:** `[YYYY-MM-DDTHH:MM:SS] LEVEL: message`(Release)。
**指纹正则:** `peer fingerprint \(SHA1\): ([A-F0-9:]+) \(SHA256\): ([A-F0-9:]+)`。

### 3.4 设置持久化(settings.rs)

**沿用 macOS 配置目录约定,但用 JSON 格式(非 Qt 私有 plist):**

```
~/Library/Application Support/com.flowdesk.app/config.json
```

**用 Rust serde 序列化为 JSON。** 不复用 Qt 的 `QSettings` 二进制 plist 格式
(那是 Qt 私有结构)。首版可读老配置是 Phase 4 的事;首版直接用新格式。

**键名设计(对齐现有 AppConfig,便于迁移):**

```rust
struct AppConfig {
    screen_name: String,
    port: u16,                 // 24800
    interface: String,         // 监听 IP
    log_level: LogLevel,
    log_to_file: bool,
    log_filename: String,
    language: String,
    crypto_enabled: bool,      // 默认 true
    require_client_certificate: bool,
    auto_hide: bool,
    auto_start: bool,
    minimize_to_tray: bool,
    // 服务端配置
    server_config: ServerConfig,
}
```

### 3.5 Tauri 命令层(commands.rs)

前端通过 `invoke('cmd_name', args)` 调用 Rust。命令清单:

| 命令 | 入参 | 返回 | 说明 |
|------|------|------|------|
| `start_server` | `{}` | `Result<()>` | 启动 barriers |
| `stop_server` | `{}` | `Result<()>` | 停止 barriers |
| `get_status` | `{}` | `State` | 查询当前状态 |
| `get_app_config` | `{}` | `AppConfig` | 读设置 |
| `save_app_config` | `AppConfig` | `Result<()>` | 写设置 |
| `get_server_config` | `{}` | `ServerConfig` | 读屏幕配置 |
| `save_server_config` | `ServerConfig` | `Result<()>` | 写屏幕配置 |
| `get_local_ips` | `{}` | `Vec<String>` | 本机 IPv4 列表 |
| `accept_fingerprint` | `{sha256}` | `Result<()>` | TLS 信任确认 |
| `open_log_file` | `{}` | `Result<()>` | 打开日志文件夹 |

**事件(Rust → 前端,通过 Tauri events):**

| 事件 | Payload | 说明 |
|------|---------|------|
| `log://line` | `{level, message}` | 一条新日志 |
| `state://change` | `State` | 状态变化 |
| `fingerprint://prompt` | `{sha1, sha256}` | 需要用户确认 TLS |

### 3.6 macOS 特定行为

复刻现有 Qt GUI 在 macOS 的关键行为:

1. **辅助功能权限(Accessibility)**
   - 启动时检查 `AXIsProcessTrustedWithOptions`
   - 若无权限,弹窗引导用户到 系统设置 → 隐私与安全性 → 辅助功能
   - **Tauri 实现:** Rust FFI 调 ApplicationServices 框架

2. **应用必须位于 `/Applications`**
   - 现有代码拒绝从 `/Volumes/`(DMG)运行
   - Tauri 同样检查,引导用户拖到 Applications

3. **Dock 显隐**
   - 隐藏主窗口时移除 Dock 图标(`TransformProcessType` 到 UIElement)
   - Tauri 通过 `app_handle.set_activation_policy()` 实现

4. **系统托盘**
   - 三态图标(Disconnected / Connected / Transfering)
   - 模板图(`-mask.png`),适配深浅色菜单栏
   - 菜单:Start / Stop / Show Log / Hide / Show / Quit
   - 双击切换窗口可见性
   - **Tauri 实现:** `tauri-plugin-system-tray`(v2 已内置)

5. **签名与公证**
   - 分发版需要 Developer ID 签名 + notarytool 公证
   - entitlements 包含 `com.apple.security.network.server`(监听 24800)

---

## 4. 阶段计划

每个 Phase 交付一个**可运行、可验证**的版本。Phase 间相互独立,随时可叫停。

### Phase 0:基建搭建(预计 1 天)

**目标:** Tauri 骨架能编译能起空窗口,确认工具链通。

**产出:**
- `src/gui-tauri/` 工程结构
- `cargo` / `npm` 依赖装好
- 一个空 Tauri 窗口能 launch
- CI 占位(后续才补 build job)

**验证:** `npm run tauri dev` 出现窗口

### Phase 1:进程监督(预计 2-3 天)★ 服务端模式优先起点

**目标:** 能通过 GUI 启动并监督 `barriers`,看到实时日志和状态。

**产出:**
- `supervisor.rs`:启动/停止/状态机/auto-restart
- `logparse.rs`:stdout 扫描
- `commands.rs`:`start_server` / `stop_server` / `get_status`
- 前端极简 UI:Start/Stop 按钮 + 状态指示 + 日志滚动区
- `settings.rs` 最小版:screenName / port / logLevel

**验证:**
- 点 Start,`barriers` 启动,日志实时滚动
- 点 Stop,`barriers` 干净退出
- 用另一台装 barrier 的机器作 client 连入,状态变为 Connected
- 杀掉 `barriers`,1 秒后自动重启

**关键里程碑:** 此时 FlowDesk 已经能替代你刚退出的 Barrier 的**服务端角色**,虽无屏幕编辑器(只能用默认单屏配置或外部 conf 文件)。

### Phase 2:配置文件生成 + 简单配置 UI(预计 2 天)

**目标:** GUI 内能编辑基础服务端配置(屏名、布局矩形),生成 barrier 文本配置。

**产出:**
- `config.rs`:生成 screens + links 文本配置
- 前端:ServerConfig 编辑表单(屏名、heartbeat、剪贴板共享等开关)
- 简单屏幕列表(非网格,Phase 3 才做网格)
- 外部 conf 文件加载

**验证:**
- 添加两个屏幕,生成配置文件,`barriers` 用它启动成功
- 另一台机器连入,键鼠跨屏工作

### Phase 3:屏幕布局编辑器(预计 3-5 天)★ 最难的 UI

**目标:** 复刻 5×3 网格拖拽编辑器。

**产出:**
- 前端:拖拽网格组件(drag-from-palette / 拖动换位 / 拖到垃圾桶删除)
- 双击编辑屏幕属性(aliases、modifiers、switchCorners)
- Ctrl+方向键 换位
- 键盘 Delete 删除
- 配置序列化覆盖全部字段(aliases / modifiers / fixes / corners)

**验证:**
- 多屏布局(2~5 屏)能保存、重启后恢复
- 生成的 barrier 配置文件与现有 Qt GUI 输出字节级兼容

### Phase 4:精修(预计 2-3 天)

**目标:** 产品级体验。

**产出:**
- 系统托盘(三态图标、菜单、双击)
- SSL/TLS 指纹对话框(解析 stdout 触发)
- Setup 向导(首启引导)
- macOS 辅助功能权限检查
- Dock 显隐
- i18n(至少 en + zh-CN)
- 应用签名与公证脚本

**验证:**
- 完整替代旧 Barrier.app 的 macOS 服务端体验
- 关机重启后自启可用

---

## 5. 风险与缓解

| 风险 | 等级 | 缓解 |
|------|------|------|
| 屏幕网格拖拽在 WebView 里实现复杂 | 中 | Phase 3 独立验证;若卡住降级为列表式编辑 |
| Tauri 系统托盘 macOS 行为与 Qt 有差异 | 低 | 用模板图标 + `set_activation_policy`,Phase 4 集中调 |
| stdout 日志扫描依赖英文子串,未来 C++ 改 i18n 会断 | 低 | C++ 核心代码不改;日志格式稳定多年 |
| 老用户 Qt 配置无法迁移 | 低 | 首版不承诺兼容;Phase 4 可加 plist 导入 |
| Tauri v2 API 在 macOS 上的边界情况 | 中 | Phase 0 先验证空骨架,早暴露问题 |

---

## 6. 许可证合规

- **Tauri 框架:** MIT/Apache-2.0 → 与 GPLv2 兼容
- **Dart/JS 前端依赖:** 必须 GPLv2 兼容(MIT/Apache/BSD/ISC)
- **新 GUI 源码:** GPLv2(继承 barrier 许可)
- **C++ 核心:** 不改动,GPLv2 头部保留
- **分发:** `.app` 内含 GPLv2 C++ 二进制 + GPLv2 Rust/JS 源码,符合 GPLv2 第 5 条

**AGENTS.md 红线遵守:**
- 不删除上游版权声明
- 新文件加 GPLv2 头
- 不引入不兼容依赖
- Tauri 源码也归 FlowDesk GPLv2

---

## 7. 决策记录(Decision Log)

| # | 决策 | 理由 |
|---|------|------|
| D1 | 用进程直启而非 IPC | macOS 不需要 IPC,简化 150 行代码与一个网络协议 |
| D2 | 新 GUI 放 `src/gui-tauri/` 与旧 `src/gui/` 共存 | 渐进替换,便于对照与回退 |
| D3 | 首版只做 macOS | 降低风险,目标用户即开发机本身 |
| D4 | 服务端模式优先 | 用户刚退出的 Barrier 是服务端角色,优先恢复该能力 |
| D5 | 不动 C++ 核心 | 接口已解耦,改核心风险高且无收益 |
| D6 | 不承诺老配置兼容 | 首版以新格式起步,迁移留 Phase 4 |
| D7 | 分 Phase 交付 | 每阶段独立可用,随时叫停不浪费投入 |

---

## 8. 决策结论(已确认)

| # | 决策项 | 选择 | 备注 |
|---|--------|------|------|
| C1 | 前端框架 | **React** | TypeScript + Vite,Tauri 官方默认栈,拖拽生态成熟 |
| C2 | 设置存储格式 | **JSON** | 路径 `~/Library/Application Support/com.flowdesk.app/config.json` |
| C3 | Bundle ID | **`com.flowdesk.app`** | 影响 plist 路径、签名、托盘标识 |
| C4 | 辅助功能权限检查 | Phase 1 集成(默认) | 服务端模式也需捕获鼠标,权限不通 barriers 跑不起来 |
| C5 | C++ 核心产物路径 | `build/bin/barriers` | Phase 0 配置 Tauri 脚本引用该路径,后续考虑固定 `dist/bin/` |
| C6 | 网格编辑器卡住的降级 | 接受降级为列表式 | Phase 3 若 3 天无突破,降级并记入风险 |
| C7 | 设计文档补充 | 暂无 | Phase 推进中按需补充 ADR |

**生效条件:** 本节填齐即视为 Phase 0 启动许可。
