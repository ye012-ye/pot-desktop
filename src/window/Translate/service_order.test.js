import assert from 'node:assert/strict';
import test from 'node:test';

import { isFirstEnabledService } from './service_order.js';

test('treats LM Studio as primary when the earlier Ollama service is disabled', () => {
    const serviceList = ['ollama@disabled', 'lmstudio@enabled'];
    const configMap = {
        'ollama@disabled': { enable: false },
        'lmstudio@enabled': { enable: true },
    };

    assert.equal(isFirstEnabledService('ollama@disabled', serviceList, configMap), false);
    assert.equal(isFirstEnabledService('lmstudio@enabled', serviceList, configMap), true);
});
