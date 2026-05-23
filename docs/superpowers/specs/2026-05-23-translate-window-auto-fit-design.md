# Translate Window Auto-Fit Height — Design

Date: 2026-05-23

## Problem

The translate window is a fixed `translate_window_width × translate_window_height` rectangle (default 350×420). When the user translates a short word, most of the window is empty. When the translation result is long, content scrolls inside the inner target area instead of the window growing. Result: the window never visually "matches" its content.

## Goal

When auto-fit is enabled, the translate window height shrinks/grows to match its rendered content on every translation update, leaving width entirely under user control.

## Non-Goals

- Animated height transitions (Tauri `setSize` is synchronous; animating would create jitter for streaming updates).
- A configurable max-height (use the current monitor's work area minus a small margin).
- Width auto-fit (text reflows when width changes; jarring during streaming).
- Per-service height memory.

## Decisions

1. **Axis:** height only. Width remains user-resizable and is still persisted by the existing `translate_remember_window_size` path.
2. **Trigger:** every content size change (first paint, source text typing, each translation service result arriving, service list reorder).
3. **UI surface:** a switch in `Config → 翻译设置`. New config key `translate_auto_fit_height`, default `true`.

## Configuration

| Key | Type | Default | Notes |
|---|---|---|---|
| `translate_auto_fit_height` | bool | `true` | New |
| `translate_window_width` | i64 | 350 | Existing — still used for initial width and as the width preserved across resizes |
| `translate_window_height` | i64 | 420 | Existing — used for initial height; ignored at runtime when auto-fit is on |
| `translate_remember_window_size` | bool | `false` | Existing — when auto-fit is on, only width is persisted on `tauri://resize` |

## Architecture

### Frontend (`src/window/Translate/index.jsx`)

Add a `useEffect` that activates when `translate_auto_fit_height === true`:

1. Pick a measurement target: `document.documentElement` (gives `scrollHeight` of the whole rendered tree).
2. Attach a `ResizeObserver` to it.
3. On every observer callback, schedule a single `requestAnimationFrame` (coalesce bursts) that runs:
   - `target = document.documentElement.scrollHeight`
   - `clamp(target, MIN, monitor.workArea.height - MARGIN)` where `MIN = 200`, `MARGIN = 80`.
   - Read current `outerSize`, convert width to logical pixels, call `appWindow.setSize(new LogicalSize(currentWidth, clampedHeight))`.
   - Skip the call if the delta is < 2 logical px (prevents fight loops with the OS frame).
4. Cleanup: disconnect observer on unmount or when the config flag flips off.

### Frontend remember-size coordination (`src/window/Translate/index.jsx`)

The existing `tauri://resize` listener writes both width and height to the store. When `translate_auto_fit_height === true`, change the listener to write only width. The listener already debounces 100ms, which is enough to dedupe the rapid setSize calls from auto-fit.

### Frontend config UI (`src/window/Config/pages/Translate/index.jsx`)

Add a `Switch` row "窗口高度自适应内容 / Auto-fit window height to content" wired to `useConfig('translate_auto_fit_height', true)`. Place it near the existing `translate_remember_window_size` switch. When the new switch is on, show a hint that the remember-size only retains width.

### Rust (`src-tauri/src/window.rs`)

In `translate_window()`, after creating/finding the window and reading stored dimensions:

- Read `translate_auto_fit_height` (default `true`).
- Call `window.set_resizable(!auto_fit)` so that with auto-fit on, the user can't fight the resize logic; with auto-fit off, current drag-to-resize behavior is preserved.

The initial size still comes from stored width/height. The frontend's first ResizeObserver tick (within ~50ms of mount) will collapse height to content. This produces a brief "tall, then collapse" flash on first open. Acceptable for v1; if reported as ugly we can later have Rust create the window with a tiny initial height when auto-fit is on, or hide-until-first-paint.

## Data Flow

```
User selects text, presses Alt+E
   ↓
Rust translate_window() creates/shows window at (W=350, H=420) logical
   ↓
React mounts, ResizeObserver attaches to <html>
   ↓
ResizeObserver fires once with current scrollHeight
   ↓
requestAnimationFrame → appWindow.setSize(W, clamp(scrollHeight))
   ↓
Translation results stream in (one TargetArea grows at a time)
   ↓
Each result → ResizeObserver fires → rAF → setSize
   ↓
User drags width edge
   ↓
tauri://resize → existing debounced handler writes width to store (skips height)
```

## Edge Cases

- **Empty source text:** clamp to MIN=200 keeps the window visible.
- **All services hidden / collapsed:** same, MIN clamp.
- **Translation result longer than screen:** clamp to `workArea.height - 80`, inner overflow scrolls as today.
- **Multi-monitor:** read `currentMonitor()` on every resize tick; don't cache.
- **User flips switch at runtime:** the existing `useConfig` hook re-runs the effect; cleanup disconnects the old observer, new effect either attaches one (on) or leaves the window at its last size (off).
- **Streaming translation services:** each token append fires ResizeObserver. rAF coalescing ensures at most one setSize per frame (~16ms).

## Out of Scope

- Touching `inline_translate_window` (no UI, doesn't need sizing).
- Touching `recognize` / `config` / `updater` windows.

## Files Changed

| File | Change |
|---|---|
| `src/window/Translate/index.jsx` | New useEffect with ResizeObserver; modify existing resize listener to skip height when auto-fit on |
| `src/window/Config/pages/Translate/index.jsx` | Switch + i18n key reference |
| `src/i18n/locales/zh_cn.json` | New string |
| `src/i18n/locales/en.json` | New string |
| `src-tauri/src/window.rs` | Read `translate_auto_fit_height`, call `set_resizable(!auto_fit)` |

Other locale files (16) get the English fallback automatically via i18next.

## Verification

Test cases via dev pot + IDEA:

1. Auto-fit on, short Chinese phrase (~5 chars) → window collapses to roughly source + each target area's minimum row height + chrome.
2. Auto-fit on, long paragraph → window grows up to monitor cap, content area scrolls past that point.
3. Auto-fit on, drag right edge → width persists across reopens (if remember-size on).
4. Auto-fit on, drag bottom edge → no-op (resizable disabled).
5. Toggle off → behavior reverts to today's fixed size, drag-to-resize works.
6. Toggle off → on → off → height-related store keys unchanged across switches.
