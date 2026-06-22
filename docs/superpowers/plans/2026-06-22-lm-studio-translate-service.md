# LM Studio Translate Service Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a first-class LM Studio translation provider with model discovery, optional API-token authentication, and OpenAI-compatible chat completions.

**Architecture:** Add a focused `lmstudio` provider whose configuration UI owns LM Studio-specific defaults and model discovery. Keep URL normalization and response parsing in a pure tested helper, then adapt the saved configuration into the existing OpenAI translation function so streaming and prompt behavior remain single-sourced.

**Tech Stack:** React 18, NextUI, Tauri HTTP API, i18next, Node `node:test`, Rust/Tauri service registry

---

## File map

- Create `src/services/translate/lmstudio/api.js`: pure URL, header, model parsing, and adapter helpers.
- Create `src/services/translate/lmstudio/api.test.js`: unit tests for the pure helper.
- Create `src/services/translate/openai/headers.js`: pure OpenAI/Azure request-header builder.
- Create `src/services/translate/openai/headers.test.js`: request-header regression tests.
- Create `src/services/translate/lmstudio/info.ts`: LM Studio service metadata and supported languages.
- Create `src/services/translate/lmstudio/index.jsx`: thin adapter into the existing OpenAI translate function.
- Create `src/services/translate/lmstudio/Config.jsx`: LM Studio configuration and model discovery UI.
- Create `public/logo/lmstudio.svg`: local provider icon.
- Modify `src/services/translate/openai/index.jsx`: use the tested header builder.
- Modify `src/services/translate/index.jsx`: export the new provider.
- Modify `src-tauri/src/config.rs`: preserve saved `lmstudio` instances.
- Modify `src/i18n/locales/zh_CN.json`: Simplified Chinese labels and errors.
- Modify `src/i18n/locales/en_US.json`: English fallback labels and errors.
- Modify `package.json`: add the focused Node test command.

### Task 1: Pure LM Studio API helpers

**Files:**
- Create: `src/services/translate/lmstudio/api.test.js`
- Create: `src/services/translate/lmstudio/api.js`
- Modify: `package.json`

- [x] **Step 1: Add a test script and write the failing helper tests**

Add this script to `package.json`:

```json
"test": "node --test src/services/translate/lmstudio/api.test.js src/services/translate/openai/headers.test.js"
```

Create `src/services/translate/lmstudio/api.test.js`:

```js
import assert from 'node:assert/strict';
import test from 'node:test';

import {
    createLmStudioHeaders,
    getChatCompletionsUrl,
    getModelsUrl,
    normalizeBaseUrl,
    parseModelIds,
    toOpenAIConfig,
} from './api.js';

test('normalizes LM Studio base URLs to one v1 segment', () => {
    assert.equal(normalizeBaseUrl('http://localhost:1234'), 'http://localhost:1234/v1');
    assert.equal(normalizeBaseUrl('http://localhost:1234/v1/'), 'http://localhost:1234/v1');
    assert.equal(
        normalizeBaseUrl('http://localhost:1234/v1/chat/completions'),
        'http://localhost:1234/v1'
    );
});

test('builds the LM Studio model and chat endpoints', () => {
    assert.equal(getModelsUrl('http://localhost:1234'), 'http://localhost:1234/v1/models');
    assert.equal(
        getChatCompletionsUrl('http://localhost:1234/v1'),
        'http://localhost:1234/v1/chat/completions'
    );
});

test('only sends bearer authorization for a non-empty token', () => {
    assert.deepEqual(createLmStudioHeaders(''), { 'Content-Type': 'application/json' });
    assert.deepEqual(createLmStudioHeaders(' token '), {
        'Content-Type': 'application/json',
        Authorization: 'Bearer token',
    });
});

test('parses model identifiers from the OpenAI-compatible response', () => {
    assert.deepEqual(parseModelIds({ data: [{ id: 'qwen' }, { id: 'gemma' }] }), ['qwen', 'gemma']);
    assert.throws(() => parseModelIds({ models: [] }), /Invalid LM Studio models response/);
});

test('adapts LM Studio configuration to the existing OpenAI provider', () => {
    const config = toOpenAIConfig({ baseUrl: 'http://localhost:1234', model: 'qwen', apiKey: '' });
    assert.equal(config.service, 'openai');
    assert.equal(config.requestPath, 'http://localhost:1234/v1/chat/completions');
    assert.equal(config.model, 'qwen');
});
```

- [x] **Step 2: Run the tests and confirm the RED state**

Run: `rtk pnpm test`

Expected: FAIL because `lmstudio/api.js` and `openai/headers.test.js` do not exist.

- [x] **Step 3: Limit the first RED run to the LM Studio test, then implement the helper**

Run: `rtk node --test src/services/translate/lmstudio/api.test.js`

Expected: FAIL with `ERR_MODULE_NOT_FOUND` for `api.js`.

Create `src/services/translate/lmstudio/api.js` with these exports:

```js
export function normalizeBaseUrl(input) {
    let value = input.trim();
    if (!/^https?:\/\//i.test(value)) value = `http://${value}`;

    const url = new URL(value);
    let pathname = url.pathname.replace(/\/+$/, '').replace(/\/chat\/completions$/i, '');
    pathname = pathname.replace(/(?:\/v1)+$/i, '/v1');
    if (!pathname.endsWith('/v1')) pathname = `${pathname}/v1`;

    url.pathname = pathname;
    url.search = '';
    url.hash = '';
    return url.href.replace(/\/$/, '');
}

export function getModelsUrl(baseUrl) {
    return `${normalizeBaseUrl(baseUrl)}/models`;
}

export function getChatCompletionsUrl(baseUrl) {
    return `${normalizeBaseUrl(baseUrl)}/chat/completions`;
}

export function createLmStudioHeaders(apiKey = '') {
    const headers = { 'Content-Type': 'application/json' };
    const token = apiKey.trim();
    if (token) headers.Authorization = `Bearer ${token}`;
    return headers;
}

export function parseModelIds(response) {
    if (!Array.isArray(response?.data)) throw new Error('Invalid LM Studio models response');
    return response.data.map((item) => item?.id).filter(Boolean);
}

export function toOpenAIConfig(config) {
    return {
        ...config,
        service: 'openai',
        requestPath: getChatCompletionsUrl(config.baseUrl),
    };
}
```

- [x] **Step 4: Run the focused LM Studio tests and confirm GREEN**

Run: `rtk node --test src/services/translate/lmstudio/api.test.js`

Expected: 5 tests pass, 0 fail.

- [x] **Step 5: Commit the helper slice**

```bash
rtk git add package.json src/services/translate/lmstudio/api.js src/services/translate/lmstudio/api.test.js
rtk git commit -m "test: cover LM Studio API helpers"
```

### Task 2: Optional OpenAI-compatible authorization

**Files:**
- Create: `src/services/translate/openai/headers.test.js`
- Create: `src/services/translate/openai/headers.js`
- Modify: `src/services/translate/openai/index.jsx:36-48`

- [x] **Step 1: Write the failing header tests**

Create `src/services/translate/openai/headers.test.js`:

```js
import assert from 'node:assert/strict';
import test from 'node:test';

import { createRequestHeaders } from './headers.js';

test('omits OpenAI authorization when the API key is empty', () => {
    assert.deepEqual(createRequestHeaders('openai', ''), { 'Content-Type': 'application/json' });
});

test('adds bearer authorization for an OpenAI-compatible API key', () => {
    assert.deepEqual(createRequestHeaders('openai', 'token'), {
        'Content-Type': 'application/json',
        Authorization: 'Bearer token',
    });
});

test('keeps the Azure api-key header', () => {
    assert.deepEqual(createRequestHeaders('azure', 'token'), {
        'Content-Type': 'application/json',
        'api-key': 'token',
    });
});
```

- [x] **Step 2: Run the header test and confirm RED**

Run: `rtk node --test src/services/translate/openai/headers.test.js`

Expected: FAIL with `ERR_MODULE_NOT_FOUND` for `headers.js`.

- [x] **Step 3: Implement the header builder and wire it into OpenAI**

Create `src/services/translate/openai/headers.js`:

```js
export function createRequestHeaders(service, apiKey = '') {
    const headers = { 'Content-Type': 'application/json' };
    if (service === 'openai') {
        const token = apiKey.trim();
        if (token) headers.Authorization = `Bearer ${token}`;
    } else {
        headers['api-key'] = apiKey;
    }
    return headers;
}
```

In `src/services/translate/openai/index.jsx`, import `createRequestHeaders` and replace the inline ternary header object with:

```js
const headers = createRequestHeaders(service, apiKey);
```

- [x] **Step 4: Run all helper tests and confirm GREEN**

Run: `rtk pnpm test`

Expected: 8 tests pass, 0 fail.

- [x] **Step 5: Commit the header slice**

```bash
rtk git add src/services/translate/openai/headers.js src/services/translate/openai/headers.test.js src/services/translate/openai/index.jsx
rtk git commit -m "fix: support optional compatible API tokens"
```

### Task 3: Provider adapter

**Files:**
- Create: `src/services/translate/lmstudio/info.ts`
- Create: `src/services/translate/lmstudio/index.jsx`

- [x] **Step 1: Create the provider metadata**

Create `src/services/translate/lmstudio/info.ts` with `info.name = 'lmstudio'`, `info.icon = 'logo/lmstudio.svg'`, and the same `Language` enum values as `src/services/translate/openai/info.ts`.

- [x] **Step 2: Create the thin translation adapter**

Create `src/services/translate/lmstudio/index.jsx`:

```jsx
import { translate as translateWithOpenAI } from '../openai';
import { toOpenAIConfig } from './api';

export async function translate(text, from, to, options = {}) {
    return translateWithOpenAI(text, from, to, {
        ...options,
        config: toOpenAIConfig(options.config),
    });
}

export * from './info';
```

- [x] **Step 3: Run unit tests**

Run: `rtk pnpm test`

Expected: 8 tests pass, 0 fail.

- [x] **Step 4: Commit the isolated adapter**

```bash
rtk git add src/services/translate/lmstudio/info.ts src/services/translate/lmstudio/index.jsx
rtk git commit -m "feat: add LM Studio translation adapter"
```

### Task 4: LM Studio configuration UI

**Files:**
- Create: `src/services/translate/lmstudio/Config.jsx`
- Create: `public/logo/lmstudio.svg`
- Modify: `src/services/translate/lmstudio/index.jsx`
- Modify: `src/services/translate/index.jsx`
- Modify: `src-tauri/src/config.rs:71-94`
- Modify: `src/i18n/locales/zh_CN.json:267-278`
- Modify: `src/i18n/locales/en_US.json:267-278`

- [x] **Step 1: Add bilingual translation keys**

Add `services.translate.lmstudio` to both locale files with these keys: `title`, `base_url`, `api_key`, `model`, `stream`, `refresh_models`, `model_loading`, `model_error`, `model_required`, `prompt_description`, and `add`.

Chinese values:

```json
{
    "title": "LM Studio",
    "base_url": "API 基础地址",
    "api_key": "API Token（可选）",
    "model": "模型",
    "stream": "流式输出",
    "refresh_models": "刷新模型",
    "model_loading": "正在读取模型...",
    "model_error": "无法读取模型，可手动填写",
    "model_required": "请先选择或填写模型",
    "prompt_description": "通过自定义 Prompt 调整翻译行为，$text $from $to $detect 会被替换为待翻译文本、源语言、目标语言和检测语言。",
    "add": "添加 Prompt"
}
```

Use equivalent English values in `en_US.json`.

- [x] **Step 2: Create the local icon**

Create `public/logo/lmstudio.svg` as a 64x64 SVG with a dark rounded square, a contrasting `LM` monogram, and no remote resources.

- [x] **Step 3: Implement configuration defaults and one-shot model discovery**

In `Config.jsx`, initialize `useConfig` with instance name `LM Studio`, `baseUrl: 'http://localhost:1234/v1'`, empty `model` and `apiKey`, `stream: false`, the existing two-message OpenAI translation prompt, and `defaultRequestArguments` imported from `../openai/Config`.

Use `useRef(false)` plus `useEffect` to call `loadModels(config)` once after the async configuration becomes available. `loadModels` must:

```jsx
const response = await fetch(getModelsUrl(config.baseUrl), {
    method: 'GET',
    headers: createLmStudioHeaders(config.apiKey),
});
if (!response.ok) throw new Error(`HTTP ${response.status}`);
setModels(parseModelIds(response.data));
```

On failure, set the localized `model_error` text without clearing `config.model`.

- [x] **Step 4: Render editable configuration fields**

Render the existing instance-name pattern, stream switch, Base URL input, password token input, model input, refresh button, prompt editor, request-arguments textarea, and save button. Attach a native `<datalist id='lmstudio-models'>` to the model input so discovered IDs are suggestions while arbitrary text remains accepted.

On submit, reject an empty trimmed model with `toast.error(model_required)`. Otherwise call `translate('hello', Language.auto, Language.zh_cn, { config })`; save with `setServiceConfig(config, true)` only after the test request succeeds.

- [x] **Step 5: Export and register the completed provider**

Add `export * from './Config';` to `src/services/translate/lmstudio/index.jsx`.

Add `import * as _lmstudio from './lmstudio';` and `export const lmstudio = _lmstudio;` to `src/services/translate/index.jsx` beside the other local-model providers.

Add `"lmstudio",` to `builtin_translate_list` in `src-tauri/src/config.rs`.

- [x] **Step 6: Verify the completed provider**

Run: `rtk pnpm test`

Expected: 8 tests pass, 0 fail.

Run: `rtk pnpm build`

Expected: Vite exits 0 and writes the production bundle.

Run from `src-tauri`: `rtk cargo fmt -- --check`

Expected: exit code 0.

- [x] **Step 7: Commit the configuration UI and registration**

```bash
rtk git add src/services/translate/lmstudio/Config.jsx src/services/translate/lmstudio/index.jsx src/services/translate/index.jsx src-tauri/src/config.rs public/logo/lmstudio.svg src/i18n/locales/zh_CN.json src/i18n/locales/en_US.json
rtk git commit -m "feat: configure LM Studio models"
```

### Task 5: Full verification and optional live smoke test

**Files:**
- Modify only files required to fix verification failures caused by this feature.

- [x] **Step 1: Inspect the final diff and formatting**

Run: `rtk git status --short`

Expected: only the implementation-plan document may remain uncommitted.

Run: `rtk git diff --check 61dbb76..HEAD`

Expected: no whitespace errors.

- [x] **Step 2: Run automated verification**

Run: `rtk pnpm test`

Expected: 8 tests pass, 0 fail.

Run: `rtk pnpm build`

Expected: exit code 0.

Run from `src-tauri`: `rtk cargo check`

Expected: exit code 0.

- [x] **Step 3: Probe LM Studio and perform a live check when available**

Request `http://localhost:1234/v1/models`. If it responds, verify that the same model IDs can be parsed and send one non-streaming translation request through the configured endpoint. If it does not respond, record the live test as unavailable rather than claiming it passed.

- [x] **Step 4: Commit the implementation plan and any verification-only correction**

```bash
rtk git add docs/superpowers/plans/2026-06-22-lm-studio-translate-service.md
rtk git commit -m "docs: add LM Studio implementation plan"
```

## Execution notes

- The test suite finished with 8 passing tests and 0 failures.
- `pnpm build` and `cargo check` completed successfully. `cargo check` retained two pre-existing unused-import warnings in `src/window.rs`.
- The repository-wide `cargo fmt -- --check` remains blocked by pre-existing formatting differences outside this change. The modified Rust file passed `rustfmt --check src/config.rs`.
- Live LM Studio verification succeeded against `http://localhost:1234/v1`: model discovery returned `hy-mt2-1.8b`, the non-streaming request translated “hello world” to “你好，世界”, and the streaming request translated “good morning” to “早上好”.
