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
    const result = await ensureFirstLmStudioModel(
        entries,
        {
            getModels: async () =>
                ++probes === 1 ? null : { models: [{ key: 'hy-mt2-1.8b', loaded_instances: [] }] },
            startServer: async (port) => {
                assert.equal(port, 1234);
                starts++;
            },
            loadModel: async (entry) => {
                assert.equal(entry.config.model, 'hy-mt2-1.8b');
                loads++;
            },
            sleep: async () => {},
        },
        { maxAttempts: 2, intervalMs: 0 }
    );

    assert.equal(result.status, 'loaded');
    assert.equal(starts, 1);
    assert.equal(loads, 1);
});

test('retries a transient model load failure', async () => {
    let loads = 0;
    let sleeps = 0;
    const result = await ensureFirstLmStudioModel(
        entries,
        {
            getModels: async () => ({ models: [{ key: 'hy-mt2-1.8b', loaded_instances: [] }] }),
            startServer: async () => assert.fail('server is already reachable'),
            loadModel: async () => {
                loads++;
                if (loads === 1) throw new Error('connection closed before message completed');
            },
            sleep: async () => sleeps++,
        },
        { loadAttempts: 2, intervalMs: 0 }
    );

    assert.equal(result.status, 'loaded');
    assert.equal(loads, 2);
    assert.equal(sleeps, 1);
});

test('does not start a local process for a remote server', async () => {
    const remote = [
        { instanceKey: 'lmstudio@remote', config: { baseUrl: 'https://models.example.com/v1', model: 'm' } },
    ];
    let starts = 0;
    const result = await ensureFirstLmStudioModel(
        remote,
        {
            getModels: async () => null,
            startServer: async () => starts++,
            loadModel: async () => assert.fail('must not load an unreachable model'),
            sleep: async () => {},
        },
        { maxAttempts: 2, intervalMs: 0 }
    );

    assert.equal(result.status, 'unreachable');
    assert.equal(starts, 0);
});

test('skips loading when the model is already in memory', async () => {
    let loads = 0;
    const result = await ensureFirstLmStudioModel(entries, {
        getModels: async () => ({
            models: [{ key: 'hy-mt2-1.8b', loaded_instances: [{ id: 'hy-mt2-1.8b' }] }],
        }),
        startServer: async () => assert.fail('server is already reachable'),
        loadModel: async () => loads++,
        sleep: async () => {},
    });

    assert.equal(result.status, 'already-loaded');
    assert.equal(loads, 0);
});
