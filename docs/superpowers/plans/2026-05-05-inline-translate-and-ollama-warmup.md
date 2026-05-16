# 原地替换翻译 + Ollama 预热 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 新增原地替换翻译快捷键 + Pot 启动时预热 Ollama 模型

**Architecture:** 复用现有翻译窗口管道（SourceArea → TargetArea），通过特殊事件前缀 `[INLINE_TRANSLATE]` 触发后台翻译，完成后通过 Rust `inline_paste` 命令模拟 Ctrl+V 粘贴替换。Ollama 预热为前端启动后调用。

**Tech Stack:** Rust (Tauri v1, enigo), React/JSX, Jotai

---

### File Structure

**Rust (new/modified):**
- `src-tauri/Cargo.toml` — add `enigo` dependency
- `src-tauri/src/window.rs` — add `inline_translate()`
- `src-tauri/src/hotkey.rs` — register inline translate hotkey
- `src-tauri/src/cmd.rs` — add `inline_paste` command
- `src-tauri/src/main.rs` — register `inline_paste` in invoke_handler

**Frontend (new/modified):**
- `src/window/Translate/components/SourceArea/index.jsx` — detect `[INLINE_TRANSLATE]` prefix, export inlineTranslateAtom
- `src/window/Translate/components/TargetArea/index.jsx` — after translation, invoke `inline_paste`
- `src/window/Config/pages/Hotkey/index.jsx` — add inline translate hotkey UI
- `src/i18n/locales/zh_CN.json` — add i18n keys
- `src/i18n/locales/en_US.json` — add i18n keys
- `src/utils/ollama_warmup.js` — new: detect and warm up Ollama
- `src/main.jsx` — call warmup after init

---

### Task 1: Add enigo dependency

**Files:**
- Modify: `src-tauri/Cargo.toml`

- [ ] **Step 1: Add enigo to Cargo.toml**

Add after line 33 (`arboard = "3.4"`):
```toml
enigo = { version = "0.2", default-features = false, features = ["x11", "wayland"] }
```

Full context:
```toml
arboard = "3.4"
enigo = { version = "0.2", default-features = false, features = ["x11", "wayland"] }
lingua = { version = "1.6.2", ...
```

- [ ] **Step 2: Verify build**

```bash
cd d:/java/code/pot-desktop && export PATH="$HOME/.cargo/bin:/c/Users/35502/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH" && cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```

Expected: successful cargo build (fetch + compile enigo).

- [ ] **Step 3: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/Cargo.lock
git commit -m "chore: add enigo dependency for keyboard simulation"
```

---

### Task 2: Add inline_translate handler and inline_paste command (Rust)

**Files:**
- Modify: `src-tauri/src/window.rs`
- Modify: `src-tauri/src/cmd.rs`

- [ ] **Step 1: Add inline_translate() to window.rs**

Add after `selection_translate()` (after line 239):

```rust
pub fn inline_translate() {
    use crate::APP;
    use tauri::Manager;

    let text = selection::get_text();
    if text.trim().is_empty() {
        return;
    }
    let app_handle = APP.get().unwrap();
    let state: tauri::State<StringWrapper> = app_handle.state();
    state.0.lock().unwrap().replace_range(.., &text);

    let window = translate_window();
    let event_text = format!("[INLINE_TRANSLATE]{}", text);
    window.emit("new_text", event_text).unwrap();
}
```

- [ ] **Step 2: Add inline_paste command to cmd.rs**

Add at end of `cmd.rs`:

```rust
#[tauri::command]
pub fn inline_paste(text: String) -> Result<(), Error> {
    use arboard::Clipboard;
    use enigo::{Enigo, Keyboard, Settings};
    use std::thread;
    use std::time::Duration;

    // Save original clipboard
    let mut old_clipboard = String::new();
    if let Ok(mut clipboard) = Clipboard::new() {
        if let Ok(old) = clipboard.get_text() {
            old_clipboard = old;
        }
        // Copy translation to clipboard
        let _ = clipboard.set_text(&text);
    }

    // Small delay to ensure clipboard is updated
    thread::sleep(Duration::from_millis(50));

    // Simulate paste: Ctrl+V (Cmd+V on macOS)
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| Error::Error(e.to_string()))?;

    #[cfg(target_os = "macos")]
    {
        enigo.key(enigo::Key::Meta, enigo::Direction::Press).unwrap();
        enigo.key(enigo::Key::Unicode('v'), enigo::Direction::Press).unwrap();
        enigo.key(enigo::Key::Unicode('v'), enigo::Direction::Release).unwrap();
        enigo.key(enigo::Key::Meta, enigo::Direction::Release).unwrap();
    }
    #[cfg(not(target_os = "macos"))]
    {
        enigo.key(enigo::Key::Control, enigo::Direction::Press).unwrap();
        enigo.key(enigo::Key::Unicode('v'), enigo::Direction::Press).unwrap();
        enigo.key(enigo::Key::Unicode('v'), enigo::Direction::Release).unwrap();
        enigo.key(enigo::Key::Control, enigo::Direction::Release).unwrap();
    }

    // Restore original clipboard after paste
    thread::sleep(Duration::from_millis(200));
    if let Ok(mut clipboard) = Clipboard::new() {
        let _ = clipboard.set_text(&old_clipboard);
    }

    Ok(())
}
```

- [ ] **Step 3: Verify cargo check passes**

```bash
export PATH="$HOME/.cargo/bin:/c/Users/35502/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH" && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10
```

Expected: `Finished dev [unoptimized + debuginfo] target(s)` — no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/window.rs src-tauri/src/cmd.rs
git commit -m "feat: add inline_translate handler and inline_paste command"
```

---

### Task 3: Register inline translate hotkey (Rust)

**Files:**
- Modify: `src-tauri/src/hotkey.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Add inline_translate to hotkey.rs**

In `register_shortcut()`, add before `_ => {}` at line 67:
```rust
"hotkey_inline_translate" => register(
    app_handle,
    "hotkey_inline_translate",
    inline_translate,
    "",
)?,
```

Update the imports at top of hotkey.rs, change line 2 to add `inline_translate`:
```rust
use crate::window::{input_translate, inline_translate, ocr_recognize, ocr_translate, selection_translate};
```

In `register()` function, add `inline_translate` case to `"all"` match arm (add before the `_ => {}` at the end of the `"all"` block):
```rust
register(
    app_handle,
    "hotkey_inline_translate",
    inline_translate,
    "",
)?;
```

In `register_shortcut_by_frontend()`, add before `_ => {}` at line 95:
```rust
"hotkey_inline_translate" => {
    register(app_handle, "hotkey_inline_translate", inline_translate, shortcut)?
}
```

- [ ] **Step 2: Register inline_paste in main.rs invoke_handler**

In `main.rs`, add `inline_paste` to the `invoke_handler![]` macro (after `font_list`):
```rust
invoke_handler(tauri::generate_handler![
    ...
    font_list,
    aliyun,
    inline_paste
])
```

The `inline_paste` import is already covered by `use cmd::*;`.

- [ ] **Step 3: Verify cargo check**

```bash
export PATH="$HOME/.cargo/bin:/c/Users/35502/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH" && cargo check --manifest-path src-tauri/Cargo.toml 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/hotkey.rs src-tauri/src/main.rs
git commit -m "feat: register inline_translate hotkey and inline_paste command"
```

---

### Task 4: Frontend — handle inline translate in SourceArea and TargetArea

**Files:**
- Modify: `src/window/Translate/components/SourceArea/index.jsx`
- Modify: `src/window/Translate/components/TargetArea/index.jsx`

- [ ] **Step 1: Add inlineTranslateAtom and handle inline prefix in SourceArea**

In SourceArea, after line 27 (`export const detectLanguageAtom = atom('');`), add:
```javascript
export const inlineTranslateAtom = atom(false);
```

Import `inlineTranslateAtom` at top:
```javascript
import { atom, useAtom } from 'jotai';
```
Change to:
```javascript
import { atom, useAtom, useSetAtom } from 'jotai';
```

In `handleNewText()`, at line 52, modify the function to detect the inline prefix. Change the beginning:

```javascript
const handleNewText = async (text) => {
    text = text.trim();
    // Check for inline translate marker
    let isInline = false;
    if (text.startsWith('[INLINE_TRANSLATE]')) {
        isInline = true;
        text = text.substring('[INLINE_TRANSLATE]'.length);
    }
    if (hideWindow) {
        appWindow.hide();
    } else if (!isInline) {
        appWindow.show();
        appWindow.setFocus();
    }
```

Then in the `[SELECTION_TRANSLATE]` branch (around line 148), after setting the source text and detecting language, add inline translate atom set. Change:

```javascript
} else {
    setWindowType('[SELECTION_TRANSLATE]');
    let newText = text.trim();
    if (deleteNewline) {
        newText = text.replace(/\-\s+/g, '').replace(/\s+/g, ' ');
    } else {
        newText = text.trim();
    }
    if (incrementalTranslate) {
        setSourceText((old) => {
            return old + ' ' + newText;
        });
    } else {
        setSourceText(newText);
    }
    detect_language(newText).then(() => {
        syncSourceText();
    });
}
```

Add after the `setSourceText(newText)` call (or `setSourceText` in incremental path), before or after `detect_language()`, set the inline atom. Actually, it's better to set the atom before syncSourceText triggers translation. Add at the top of the else block:

```javascript
} else {
    setWindowType('[SELECTION_TRANSLATE]');
    if (isInline) {
        setInlineTranslateAtom(true);
    }
    // ... rest stays the same
```

Wait, we need `setInlineTranslateAtom`. Add the setter in the component's destructuring. In the component function, add:
```javascript
const setInlineTranslate = useSetAtom(inlineTranslateAtom);
```

And in the `else` block, add `if (isInline) { setInlineTranslate(true); }`.

- [ ] **Step 2: Call inline_paste after translation in TargetArea**

In TargetArea, import at the top:
```javascript
import { invoke } from '@tauri-apps/api';
```
(already imported, verify it's there)

Add import for inlineTranslateAtom:
```javascript
import { sourceTextAtom, detectLanguageAtom, inlineTranslateAtom } from '../SourceArea';
```

In the component, add:
```javascript
const inlineTranslate = useAtomValue(inlineTranslateAtom);
const setInlineTranslate = useSetAtom(inlineTranslateAtom);
```

In the `translate()` function, in the plugin translate resolve callback (around line 192-208), after:
```javascript
setResult(typeof v === 'string' ? v.trim() : v);
setIsLoading(false);
```

Add (inside plugin branch and builtin branch):
```javascript
if (inlineTranslate) {
    setInlineTranslate(false);
    const resultText = typeof v === 'string' ? v.trim() : v;
    if (resultText) {
        invoke('inline_paste', { text: resultText });
    }
}
```

Same for the plugin branch (around line 192-231) and the builtin branch (around line 264-313).

- [ ] **Step 3: Verify frontend builds**

```bash
cd d:/java/code/pot-desktop && pnpm build 2>&1 | tail -10
```

Expected: `✓ built in ...` — no errors.

- [ ] **Step 4: Commit**

```bash
git add src/window/Translate/components/SourceArea/index.jsx src/window/Translate/components/TargetArea/index.jsx
git commit -m "feat: add inline translate detection and paste callback in translate pipeline"
```

---

### Task 5: Add inline translate hotkey UI in Config

**Files:**
- Modify: `src/window/Config/pages/Hotkey/index.jsx`

- [ ] **Step 1: Add inlineTranslate state and UI section**

In `Hotkey/index.jsx`, add after line 52:
```javascript
const [inlineTranslate, setInlineTranslate] = useConfig('hotkey_inline_translate', '');
```

Add a new hotkey config section after the last `<div className='config-item'>` (after the ocr_translate div, around line 242):

```jsx
<div className='config-item'>
    <h3 className='my-auto'>{t('config.hotkey.inline_translate')}</h3>
    {inlineTranslate !== null && (
        <Input
            type='hotkey'
            variant='bordered'
            value={inlineTranslate}
            label={t('config.hotkey.set_hotkey')}
            className='max-w-[50%]'
            onKeyDown={(e) => {
                keyDown(e, setInlineTranslate);
            }}
            onFocus={() => {
                unregister(inlineTranslate);
                setInlineTranslate('');
            }}
            endContent={
                <Button
                    size='sm'
                    variant='flat'
                    className={`${inlineTranslate === '' && 'hidden'}`}
                    onPress={() => {
                        registerHandler('hotkey_inline_translate', inlineTranslate);
                    }}
                >
                    {t('common.ok')}
                </Button>
            }
        />
    )}
</div>
```

- [ ] **Step 2: Commit**

```bash
git add src/window/Config/pages/Hotkey/index.jsx
git commit -m "feat: add inline translate hotkey config UI"
```

---

### Task 6: Add i18n keys

**Files:**
- Modify: `src/i18n/locales/zh_CN.json`
- Modify: `src/i18n/locales/en_US.json`

- [ ] **Step 1: Add Chinese i18n key**

In `zh_CN.json`, in the `"hotkey"` section (after line 152, after `"ocr_translate": "截图翻译"`), add:
```json
"inline_translate": "原地替换翻译",
```

- [ ] **Step 2: Add English i18n key**

In `en_US.json`, in the `"hotkey"` section (after line 153, after `"ocr_translate": "Screenshot Translation"`), add:
```json
"inline_translate": "Inline Replace Translation",
```

- [ ] **Step 3: Commit**

```bash
git add src/i18n/locales/zh_CN.json src/i18n/locales/en_US.json
git commit -m "feat: add i18n keys for inline translate hotkey"
```

---

### Task 7: Add Ollama warmup utility

**Files:**
- Create: `src/utils/ollama_warmup.js`
- Modify: `src/main.jsx`

- [ ] **Step 1: Create ollama_warmup.js**

```javascript
import { info, warn } from 'tauri-plugin-log-api';
import { store } from './store';

export async function warmupOllama() {
    try {
        await store.load();
        const translateServiceList = await store.get('translate_service_list');

        if (!translateServiceList || !translateServiceList.value) {
            return;
        }

        const serviceList = translateServiceList.value;
        let ollamaModel = null;
        let ollamaHost = 'http://localhost:11434';

        // Check each translate service instance for Ollama
        for (const instanceKey of serviceList) {
            const config = await store.get(instanceKey);
            if (!config || !config.value) continue;

            const cfg = config.value;
            const serviceName = instanceKey.split('@')[0];

            // Check if this is an ollama service or openai-compatible pointing to Ollama
            if (serviceName === 'ollama' || cfg.service === 'ollama') {
                ollamaModel = cfg.model || 'llama3';
                ollamaHost = cfg.requestPath || 'http://localhost:11434';
                break;
            }
            // Also check openai-compatible services that might point to Ollama
            if ((cfg.service === 'openai' || serviceName === 'openai') && cfg.requestPath) {
                const path = cfg.requestPath;
                if (path.includes('11434') || path.includes('ollama')) {
                    ollamaModel = cfg.model || 'llama3';
                    ollamaHost = path;
                    break;
                }
            }
        }

        if (!ollamaModel) {
            return;
        }

        // Ensure host has scheme
        if (!ollamaHost.startsWith('http')) {
            ollamaHost = 'http://' + ollamaHost;
        }

        info(`Warming up Ollama model: ${ollamaModel} at ${ollamaHost}`);

        const { Ollama } = await import('ollama');
        const ollama = new Ollama({ host: ollamaHost });

        await ollama.chat({
            model: ollamaModel,
            messages: [{ role: 'user', content: 'hi' }],
            stream: false,
            keep_alive: -1,
        });

        info('Ollama warmup complete');
    } catch (e) {
        warn(`Ollama warmup failed: ${e}`);
    }
}
```

- [ ] **Step 2: Call warmup from main.jsx**

In `main.jsx`, after line 7, add import:
```javascript
import { warmupOllama } from './utils/ollama_warmup';
```

In the `initStore().then(async () => { ... })` block, add the warmup call after `await initEnv();`:
```javascript
initStore().then(async () => {
    await initEnv();
    warmupOllama(); // fire-and-forget, don't await
    const rootElement = document.getElementById('root');
    // ... rest stays the same
});
```

- [ ] **Step 3: Verify build**

```bash
cd d:/java/code/pot-desktop && pnpm build 2>&1 | tail -10
```

Expected: successful build, no errors.

- [ ] **Step 4: Commit**

```bash
git add src/utils/ollama_warmup.js src/main.jsx
git commit -m "feat: add Ollama model warmup on startup"
```

---

### Task 8: End-to-end build verification

- [ ] **Step 1: Full Tauri build**

```bash
export PATH="$HOME/.cargo/bin:/c/Users/35502/AppData/Local/Microsoft/WinGet/Packages/BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe/mingw64/bin:$PATH" && cd d:/java/code/pot-desktop && pnpm tauri build 2>&1 | tail -20
```

Expected: successful build producing installer artifacts.

- [ ] **Step 2: Commit any lockfile changes**

```bash
git add -A
git commit -m "chore: update lockfiles after build verification"
```
