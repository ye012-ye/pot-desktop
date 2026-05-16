# Design: 原地替换翻译 + Ollama 启动预热

Date: 2026-05-05
Status: approved

---

## Feature 1: 原地替换翻译

### Summary

新增独立全局快捷键，用户选中文本后按快捷键，原文在原应用中直接被替换为译文。

现有划词翻译（弹出翻译窗口）不受影响，两个快捷键共存。

### Flow

```
用户选中文本 → 按新快捷键 → 原文被替换为译文（在原应用中）
```

### Architecture

#### 1.1 Rust: new hotkey handler `inline_translate()`

File: `src-tauri/src/window.rs`

- Reads selected text via `selection::get_text()`
- Saves current clipboard content
- Emits a Tauri event to trigger translation (marker: `"[INLINE_TRANSLATE]"`), no window shown
- New window helper: `inline_translate_window()` — creates/finds a hidden "inline_translate" window for running the translation pipeline

#### 1.2 Frontend: hidden translation execution

File: `src/App.jsx` — add `inline_translate` to windowMap

New component: `src/window/InlineTranslate/index.jsx`

- Receives the translation request event
- Runs translation using the currently active translate service (same pipeline as Translate window)
- Does NOT show any window (stays invisible)
- On translation complete: calls `invoke('inline_paste', { text })`

#### 1.3 Rust: new command `inline_paste`

File: `src-tauri/src/cmd.rs`

- Receives translated text
- Copies translation to clipboard
- Simulates Ctrl+V / Cmd+V keyboard input (via `enigo` crate)
- Restores original clipboard after ~200ms delay

#### 1.4 New hotkey registration

- Config key: `hotkey_inline_translate` (default: empty, user configures)
- Registered alongside existing 4 hotkeys in `hotkey.rs`
- Frontend hotkey config page: add entry for this new hotkey

#### 1.5 New dependency

- `enigo` crate (cross-platform keyboard/mouse simulation, MIT license)

### Files to modify (Rust)

| File | Change |
|---|---|
| `src-tauri/Cargo.toml` | Add `enigo` dependency |
| `src-tauri/src/window.rs` | Add `inline_translate()` handler, `inline_translate_window()` |
| `src-tauri/src/hotkey.rs` | Register `hotkey_inline_translate` |
| `src-tauri/src/cmd.rs` | Add `inline_paste` command |
| `src-tauri/src/main.rs` | Register `inline_paste` command in invoke_handler |

### Files to modify (Frontend)

| File | Change |
|---|---|
| `src/App.jsx` | Add `inline_translate` window route |
| `src/window/InlineTranslate/index.jsx` | New component — hidden translation executor |
| `src/window/Config/pages/Hotkey/index.jsx` | Add inline translate hotkey config UI |
| `src/i18n/locales/zh_CN.json` | Add i18n keys |
| `src/i18n/locales/en_US.json` | Add i18n keys |

---

## Feature 2: Pot 启动时预热 Ollama

### Summary

Pot 启动完成后，自动检测是否配置了 Ollama 翻译服务。若有，发送一个轻量级请求预热 Ollama 模型，确保首次翻译不卡顿。

### Flow

```
Pot 启动 → 加载配置 → 检测 Ollama 服务 → 发送预热请求 → 静默完成
```

### Architecture

File: `src/main.jsx` or `src/App.jsx`

After `initStore()` and config load:

1. Read config keys for translate service instances
2. Check if any instance uses Ollama (`ollama` or `openai`-compatible service pointing to Ollama)
3. If so, call `ollama.chat({ model, messages: [{role:'user', content:'hi'}], stream: false })`
4. Result is ignored — only purpose is to trigger model loading
5. On error: log via `warn()` but do not show notification

No new files needed. A utility function in `src/utils/ollama_warmup.js` would suffice.

### Files to modify (Frontend)

| File | Change |
|---|---|
| `src/utils/ollama_warmup.js` | New utility — detect + warm up Ollama |
| `src/main.jsx` | Call warmup after init |

---

## Scope Boundaries

**Out of scope:**
- Replacing visible translate window (existing feature unchanged)
- Polling-based automatic selection detection
- Config UI for warmup (it's automatic and silent)
- Error notifications for warmup failure
