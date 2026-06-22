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
