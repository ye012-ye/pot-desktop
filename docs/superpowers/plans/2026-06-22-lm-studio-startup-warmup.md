# LM Studio Startup Warmup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Start the configured local LM Studio server when Pot launches and explicitly preload the first configured LM Studio translation model without blocking the UI.

**Architecture:** Keep selection, URL derivation, local-host detection, loaded-state checks, and the retry workflow in a dependency-injected JavaScript core that is testable with Node. A thin Tauri adapter performs store, HTTP, logging, and invoke operations, while one Rust command locates `lms` and starts the configured port without a console window.

**Tech Stack:** React/Vite startup, Tauri 1 HTTP and invoke APIs, Node `node:test`, Rust `std::process::Command`, LM Studio CLI and REST API

---

## File map

- Create `src/utils/lmstudio_warmup_core.js`: pure selection, endpoint, loaded-state, and retry workflow.
- Create `src/utils/lmstudio_warmup.test.js`: Node tests for the complete startup decision flow.
- Create `src/utils/lmstudio_warmup.js`: Tauri store, HTTP, invoke, and logging adapter.
- Modify `package.json`: include the new warmup test.
- Modify `src-tauri/src/cmd.rs`: build and spawn the LM Studio CLI command plus focused Rust tests.
- Modify `src-tauri/src/main.rs`: register the new Tauri command.
- Modify `src/main.jsx`: start LM Studio warmup without awaiting it.

### Task 1: Testable warmup decision core

**Files:**
- Create: `src/utils/lmstudio_warmup.test.js`
- Create: `src/utils/lmstudio_warmup_core.js`
- Modify: `package.json`

- [x] **Step 1: Add the test file to the test command**

Set the `test` script to:

```json
"test": "node --test src/services/translate/lmstudio/api.test.js src/services/translate/openai/headers.test.js src/utils/lmstudio_warmup.test.js"
```

- [x] **Step 2: Write failing workflow tests**

Create `src/utils/lmstudio_warmup.test.js` with tests that import these not-yet-created exports:

```js
import assert from 'node:assert/strict';
import test from 'node:test';

import {
    ensureFirstLmStudioModel,
    getManagementTarget,
    isModelLoaded,
    selectFirstLmStudioEntry,
} from './lmstudio_warmup_core.js';

const entries = [
    { instanceKey: 'openai@cloud', config: { model: 'cloud' } },
    {
        instanceKey: 'lmstudio@first',
        config: { baseUrl: 'http://localhost:1234/v1', model: 'hy-mt2-1.8b', apiKey: '' },
    },
    {
        instanceKey: 'lmstudio@second',
        config: { baseUrl: 'http://localhost:1234/v1', model: 'second', apiKey: '' },
    },
];

test('selects only the first configured LM Studio entry', () => {
    assert.equal(selectFirstLmStudioEntry(entries).instanceKey, 'lmstudio@first');
});

test('derives native management URLs and local port from the OpenAI base URL', () => {
    assert.deepEqual(getManagementTarget('http://localhost:1234/v1'), {
        isLocal: true,
        port: 1234,
        modelsUrl: 'http://localhost:1234/api/v1/models',
        loadUrl: 'http://localhost:1234/api/v1/models/load',
    });
    assert.equal(getManagementTarget('https://models.example.com/v1').isLocal, false);
});

test('recognizes a model key or instance identifier as loaded', () => {
    const response = {
        models: [{ key: 'hy-mt2-1.8b', loaded_instances: [{ id: 'translator' }] }],
    };
    assert.equal(isModelLoaded(response, 'hy-mt2-1.8b'), true);
    assert.equal(isModelLoaded(response, 'translator'), true);
    assert.equal(isModelLoaded(response, 'other'), false);
});

test('starts a local server after a failed probe and loads the configured model', async () => {
    let probes = 0;
    let starts = 0;
    let loads = 0;
    const result = await ensureFirstLmStudioModel(entries, {
        getModels: async () => (++probes === 1 ? null : { models: [{ key: 'hy-mt2-1.8b', loaded_instances: [] }] }),
        startServer: async (port) => {
            assert.equal(port, 1234);
            starts++;
        },
        loadModel: async (entry) => {
            assert.equal(entry.config.model, 'hy-mt2-1.8b');
            loads++;
        },
        sleep: async () => {},
    }, { maxAttempts: 2, intervalMs: 0 });

    assert.equal(result.status, 'loaded');
    assert.equal(starts, 1);
    assert.equal(loads, 1);
});

test('does not start a local process for a remote server', async () => {
    const remote = [{ instanceKey: 'lmstudio@remote', config: { baseUrl: 'https://models.example.com/v1', model: 'm' } }];
    let starts = 0;
    const result = await ensureFirstLmStudioModel(remote, {
        getModels: async () => null,
        startServer: async () => starts++,
        loadModel: async () => assert.fail('must not load an unreachable model'),
        sleep: async () => {},
    }, { maxAttempts: 2, intervalMs: 0 });

    assert.equal(result.status, 'unreachable');
    assert.equal(starts, 0);
});

test('skips loading when the model is already in memory', async () => {
    let loads = 0;
    const result = await ensureFirstLmStudioModel(entries, {
        getModels: async () => ({ models: [{ key: 'hy-mt2-1.8b', loaded_instances: [{ id: 'hy-mt2-1.8b' }] }] }),
        startServer: async () => assert.fail('server is already reachable'),
        loadModel: async () => loads++,
        sleep: async () => {},
    });

    assert.equal(result.status, 'already-loaded');
    assert.equal(loads, 0);
});
```

- [x] **Step 3: Run the new test and confirm RED**

Run: `rtk node --test src/utils/lmstudio_warmup.test.js`

Expected: FAIL with `ERR_MODULE_NOT_FOUND` for `lmstudio_warmup_core.js`.

- [x] **Step 4: Implement the minimal core**

Create `src/utils/lmstudio_warmup_core.js`:

```js
import { normalizeBaseUrl } from '../services/translate/lmstudio/api.js';

export function selectFirstLmStudioEntry(entries) {
    return entries.find(({ instanceKey, config }) => instanceKey.split('@')[0] === 'lmstudio' && config?.model);
}

export function getManagementTarget(baseUrl) {
    const apiUrl = new URL(normalizeBaseUrl(baseUrl));
    const isLocal = ['localhost', '127.0.0.1', '::1'].includes(apiUrl.hostname.toLowerCase());
    const port = Number(apiUrl.port || (apiUrl.protocol === 'https:' ? 443 : 80));
    return {
        isLocal,
        port,
        modelsUrl: `${apiUrl.origin}/api/v1/models`,
        loadUrl: `${apiUrl.origin}/api/v1/models/load`,
    };
}

export function isModelLoaded(response, model) {
    if (!Array.isArray(response?.models)) throw new Error('Invalid LM Studio native models response');
    return response.models.some(
        (item) =>
            (item.key === model && item.loaded_instances?.length > 0) ||
            item.loaded_instances?.some((instance) => instance.id === model)
    );
}

export async function ensureFirstLmStudioModel(entries, dependencies, options = {}) {
    const entry = selectFirstLmStudioEntry(entries);
    if (!entry) return { status: 'skipped' };

    const target = getManagementTarget(entry.config.baseUrl);
    const maxAttempts = options.maxAttempts ?? 60;
    const intervalMs = options.intervalMs ?? 500;
    let models = await dependencies.getModels(entry, target);

    if (!models && target.isLocal) await dependencies.startServer(target.port);
    for (let attempt = 0; !models && attempt < maxAttempts; attempt++) {
        await dependencies.sleep(intervalMs);
        models = await dependencies.getModels(entry, target);
    }
    if (!models) return { status: 'unreachable', entry, target };
    if (isModelLoaded(models, entry.config.model)) return { status: 'already-loaded', entry, target };

    await dependencies.loadModel(entry, target);
    return { status: 'loaded', entry, target };
}
```

- [x] **Step 5: Run tests and confirm GREEN**

Run: `rtk pnpm test`

Expected: 14 tests pass, 0 fail.

- [x] **Step 6: Commit the tested core**

```bash
rtk git add package.json src/utils/lmstudio_warmup_core.js src/utils/lmstudio_warmup.test.js
rtk git commit -m "test: cover LM Studio startup warmup"
```

### Task 2: Tauri command for starting the LM Studio server

**Files:**
- Modify: `src-tauri/src/cmd.rs:221-270`
- Modify: `src-tauri/src/main.rs:150-180`

- [x] **Step 1: Add a failing Rust command-construction test**

Add a `#[cfg(test)]` module beside the new helper declarations in `cmd.rs`:

```rust
#[cfg(test)]
mod lmstudio_tests {
    use super::lmstudio_server_command;
    use std::path::Path;

    #[test]
    fn lmstudio_server_command_uses_requested_port() {
        let command = lmstudio_server_command(Path::new("lms"), 1234);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(args, ["server", "start", "--port", "1234"]);
    }
}
```

- [x] **Step 2: Run the focused Rust test and confirm RED**

Run from `src-tauri`: `rtk cargo test lmstudio_server_command_uses_requested_port`

Expected: compilation FAIL because `lmstudio_server_command` does not exist.

- [x] **Step 3: Implement command creation and spawning**

Add private helpers that return candidates (`lms` and `$HOME/.lmstudio/bin/lms[.exe]`) and construct `lms server start --port <port>` with null stdio. On Windows apply `CREATE_NO_WINDOW | DETACHED_PROCESS`.

Add this Tauri command:

```rust
#[tauri::command]
pub fn start_lmstudio_server(port: u16) -> Result<(), Error> {
    let mut last_error = None;
    for executable in lmstudio_cli_candidates() {
        match lmstudio_server_command(&executable, port).spawn() {
            Ok(_) => {
                info!("Spawned LM Studio server on port {} via {:?}", port, executable);
                return Ok(());
            }
            Err(error) => {
                info!("Skip {:?}: {}", executable, error);
                last_error = Some(error);
            }
        }
    }
    Err(Error::Io(last_error.unwrap_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "LM Studio CLI was not found")
    })))
}
```

Register `start_lmstudio_server` in `tauri::generate_handler!` in `src-tauri/src/main.rs`.

- [x] **Step 4: Run focused Rust and JavaScript tests**

Run from `src-tauri`: `rtk cargo test lmstudio_server_command_uses_requested_port`

Expected: 1 focused test passes.

Run from repository root: `rtk pnpm test`

Expected: 14 tests pass, 0 fail.

- [x] **Step 5: Commit the Tauri command**

```bash
rtk git add src-tauri/src/cmd.rs src-tauri/src/main.rs
rtk git commit -m "feat: start LM Studio server from Pot"
```

### Task 3: Runtime adapter and startup wiring

**Files:**
- Create: `src/utils/lmstudio_warmup.js`
- Modify: `src/main.jsx:7-22`

- [x] **Step 1: Implement the Tauri adapter**

Create `src/utils/lmstudio_warmup.js` that:

- loads `translate_service_list` and the listed LM Studio instance configurations from `store`;
- calls `ensureFirstLmStudioModel` with real dependencies;
- performs model probes with Tauri HTTP `GET` and validates `data.models`;
- calls `invoke('start_lmstudio_server', { port })` only through the core decision;
- calls `POST /api/v1/models/load` with `Body.json({ model })`;
- reuses `createLmStudioHeaders(apiKey)`;
- logs `loaded`, `already-loaded`, `unreachable`, and failure outcomes;
- keeps one module-level in-flight Promise and catches every failure.

- [x] **Step 2: Wire startup without blocking render**

Import `warmupLMStudio` in `src/main.jsx` and call it next to the existing Ollama warmup:

```jsx
warmupOllama();
warmupLMStudio();
```

Do not await either warmup before `root.render(...)`.

- [x] **Step 3: Run automated verification**

Run: `rtk pnpm test`

Expected: 14 tests pass, 0 fail.

Run: `rtk pnpm build`

Expected: Vite exits 0.

Run from `src-tauri`: `rtk cargo check`

Expected: exit code 0, allowing the two pre-existing unused-import warnings in `src/window.rs`.

- [x] **Step 4: Commit runtime wiring**

```bash
rtk git add src/utils/lmstudio_warmup.js src/main.jsx
rtk git commit -m "feat: preload LM Studio model at startup"
```

### Task 4: Live restart and load verification

**Files:**
- Modify only files required to fix failures caused by this feature.

- [x] **Step 1: Record the configured target**

Confirm that the first configured LM Studio service uses `localhost:1234` and a valid model before stopping the server.

- [x] **Step 2: Restart the local development flow**

Stop the LM Studio server with `rtk lms server stop`, then restart Pot development mode so the startup hook runs. Confirm the Rust log reports spawning the CLI and the JavaScript log reports a loaded or already-loaded outcome.

- [x] **Step 3: Verify the live state and translation**

Request `http://localhost:1234/api/v1/models` and confirm the configured model has a non-empty `loaded_instances`. Send one non-streaming request to `/v1/chat/completions` and confirm a translation is returned.

- [x] **Step 4: Final checks and plan commit**

Run `rtk git diff --check 92b1891..HEAD`, `rtk git status --short`, and the full test/build/check commands again. Mark completed checkboxes and add execution evidence to this plan, then commit it:

```bash
rtk git add docs/superpowers/plans/2026-06-22-lm-studio-startup-warmup.md
rtk git commit -m "docs: add LM Studio startup warmup plan"
```

## Execution evidence

- JavaScript RED: `ERR_MODULE_NOT_FOUND` for `lmstudio_warmup_core.js` before the core existed.
- Rust RED: unresolved import `super::lmstudio_server_command` before the helper existed.
- JavaScript GREEN: 14 tests passed, 0 failed.
- Rust GREEN: `lmstudio_server_command_uses_requested_port` passed.
- Build checks: `pnpm build` and `cargo check` exited successfully; `cargo check` retained two pre-existing unused-import warnings in `src/window.rs`.
- Cold-start check: with the LM Studio server stopped and `hy-mt2-1.8b` unloaded, Pot logged `Spawned LM Studio server on port 1234 via "lms"` and then confirmed the model was loaded.
- Native API check: `/api/v1/models` reported one loaded instance for `hy-mt2-1.8b`.
- Translation check: a non-streaming `/v1/chat/completions` request translated `Hello world` to `你好世界` with finish reason `stop`.
