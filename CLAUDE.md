# CLAUDE.md

此文件为 Claude Code (claude.ai/code) 在此仓库中工作时提供指导。

## 构建与开发

```bash
# 安装依赖
pnpm install

# 开发模式（Vite + Tauri 热重载）
pnpm tauri dev

# 构建桌面应用
pnpm tauri build

# 仅前端开发（浏览器）
pnpm dev

# 代码格式化
pnpm prettier --write .
pnpm prettier --check .
```

本项目没有测试套件，也没有测试基础设施——修改代码时不需要编写测试。

## 架构

**Pot** 是一款跨平台的翻译/OCR/TTS 桌面应用（支持 Windows、macOS、Linux）。

### 技术栈

- **桌面外壳：** Tauri v1（Rust 后端 + webview 前端）
- **前端：** React 18 + Vite 5 + JSX（组件不使用 TypeScript；`.ts` 文件仅用于类型定义和服务元数据）
- **状态管理：** Jotai（atoms），通过 `tauri-plugin-store` 持久化为磁盘上的 JSON 文件
- **UI：** NextUI v2（React 组件库）+ Tailwind CSS
- **国际化：** `react-i18next`，支持 18 种语言
- **包管理器：** pnpm

### 多窗口架构

应用使用多个 Tauri 窗口，每个窗口有唯一的 label。[App.jsx](src/App.jsx) 根据 `appWindow.label` 将窗口映射到对应的 React 组件：

| 窗口 Label | 组件 | 入口文件 | 用途 |
|---|---|---|---|
| `translate` | `Translate` | `index.html` | 主翻译面板（输入 + 输出） |
| `screenshot` | `Screenshot` | `index.html` | 截图捕获覆盖层 |
| `recognize` | `Recognize` | `index.html` | OCR 图片识别 |
| `config` | `Config` | `index.html` | 设置/偏好窗口 |
| `updater` | `Updater` | `index.html` | 更新对话框 |
| `daemon` | — | `daemon.html` | 隐藏的后台进程（保持应用存活） |

daemon 窗口永远不可见——它的存在仅仅是为了在所有可见窗口关闭时，应用不会退出。应用在 `main.rs` 中通过 `ExitRequested::prevent_exit()` 阻止退出。

### Rust 后端（`src-tauri/src/`）

| 模块 | 用途 |
|---|---|
| `main.rs` | 应用初始化、插件注册、系统托盘、全局快捷键、HTTP 服务启动 |
| `cmd.rs` | Tauri 命令（JS 端通过 `invoke()` 调用）—— `get_text`、`cut_image`、`system_ocr`、`set_proxy`、`install_plugin`、`lang_detect` 等 |
| `window.rs` | 窗口创建/显示/隐藏辅助函数（`config_window`、`updater_window`、`translate_window` 等） |
| `hotkey.rs` | 全局快捷键注册/注销 |
| `clipboard.rs` | 剪贴板监听（文本和图片） |
| `screenshot.rs` | 使用平台特定 API 进行屏幕截图 |
| `system_ocr.rs` | Windows 系统 OCR 集成 |
| `config.rs` | 配置存储读写（`.config.json`），首次运行检测 |
| `lang_detect.rs` | 通过 `lingua` Rust 库进行语言检测 |
| `server.rs` | 本地 HTTP 服务（`tiny_http`），用于外部应用集成（回环 HTTP API） |
| `updater.rs` | 自动更新检查 |
| `backup.rs` | 通过 WebDAV 和阿里云 OSS 进行配置备份/恢复 |
| `tray.rs` | 系统托盘菜单 |
| `error.rs` | 错误类型 |

**全局状态模式：** `APP` 是一个 `OnceCell<tauri::AppHandle>`，在初始化时设置。`StringWrapper` 通过 `Mutex<String>` 持有剪贴板文本。

命令在 `main.rs` 的 `invoke_handler![]` 中注册——添加新的 Rust 命令需要同时实现该命令并在此处注册。

### 前端服务插件系统

各类服务（翻译、OCR/识别、TTS、生词本）遵循一致的插件模式，目录结构为 `src/services/<类型>/<提供商>/`：

```
src/services/translate/openai/
├── info.ts        # 元数据：名称、图标、支持的语言
├── Config.jsx     # 设置界面组件 + 默认配置
└── index.jsx      # 导出 `translate()` 函数
```

- **内置服务**在 `src/services/<类型>/index.jsx` 中导入并注册
- **外部插件**在运行时安装——前端 `install_plugin` 从 URL 获取/解压 zip 包，后端 `install_plugin` 命令负责复制文件。插件是动态加载的
- 每种服务类型有统一的接口：
  - `translate(text, from, to, options)` —— 翻译服务
  - `recognize(img, from, to, options)` —— OCR 服务
  - `tts(text, from, options)` —— TTS 服务
  - `collect(data, options)` —— 生词本/导出服务

### 配置窗口路由

配置窗口使用 `react-router-dom`，嵌套路由定义在 [src/window/Config/routes/index.jsx](src/window/Config/routes/index.jsx)。页面位于 `src/window/Config/pages/`：

- `General/` —— 主题、语言、字体、透明度
- `Translate/` —— 翻译设置和界面配置
- `Recognize/` —— OCR 设置
- `Service/` —— 按 Translate/Recognize/Tts/Collection 分类，每类都有 `SelectModal`（添加服务）和 `ConfigModal`（配置服务实例）
- `Hotkey/` —— 键盘快捷键绑定
- `History/` —— 翻译历史
- `Backup/` —— WebDAV/阿里云/本地备份
- `About/` —— 关于信息

### 状态管理（Jotai Atoms）

配置通过 `useConfig(key, defaultValue)` hook 读写，该 hook 封装了 Jotai atoms 并与 Rust 配置库同步。全局配置存储是由 `tauri-plugin-store` 管理的单个 JSON 文件。配置文件上设置了监听，当外部修改时会重新加载并触发 `reload_store`。

### 关键依赖

- `jotai` —— 状态管理（带持久化的 atoms）
- `@nextui-org/react` —— UI 组件库
- `tailwindcss` —— 原子化 CSS
- `react-i18next` —— 多语言翻译
- `crypto-js` —— 配置中 API 密钥的加密
- `jsqr` —— 二维码识别
- `tesseract.js` —— 浏览器端 OCR 回退方案
- `jose` —— 云端 API 认证的 JWT 处理
- `react-markdown` —— 翻译输出中的 Markdown 渲染
