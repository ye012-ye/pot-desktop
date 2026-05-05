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

        const { Ollama } = await import('ollama/browser');
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
