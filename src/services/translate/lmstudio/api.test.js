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

test('removes line breaks from LM Studio prompt templates', () => {
    const config = toOpenAIConfig({
        baseUrl: 'http://localhost:1234',
        promptList: [
            { role: 'system', content: '专业翻译机器人\n\n' },
            { role: 'user', content: '把以下内容从 \r\n$from \n翻译成 $to。\n\n$text' },
        ],
    });

    assert.deepEqual(config.promptList, [
        { role: 'system', content: '专业翻译机器人' },
        { role: 'user', content: '把以下内容从 $from 翻译成 $to。$text' },
    ]);
});
