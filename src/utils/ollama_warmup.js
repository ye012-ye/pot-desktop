import { info, warn } from 'tauri-plugin-log-api';
import { invoke } from '@tauri-apps/api';
import { store } from './store';

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// 探测 ollama 服务是否就绪。Tauri 的 fetch 不受 CORS 限制，走 Rust。
async function isOllamaUp(host) {
    try {
        const { fetch } = await import('@tauri-apps/api/http');
        const res = await fetch(`${host}/api/tags`, { method: 'GET', timeout: 2 });
        return res.ok;
    } catch {
        return false;
    }
}

// 翻译调用前先确保 ollama 服务在跑。已经在跑就立刻返回 true。
// 失败 / 超时返回 false，让上层选择继续抛错或自行兜底。
let inflightEnsure = null;
export async function ensureOllamaReady(host = 'http://localhost:11434') {
    if (!host.startsWith('http')) host = 'http://' + host;
    if (await isOllamaUp(host)) return true;

    // 同一时间只允许一次 spawn + 轮询，避免并发翻译时反复触发
    if (!inflightEnsure) {
        inflightEnsure = (async () => {
            info(`ensureOllamaReady: spawning ollama serve for ${host}`);
            try {
                await invoke('start_ollama_serve');
            } catch (e) {
                warn(`start_ollama_serve invoke failed: ${e}`);
            }
            for (let i = 0; i < 20; i++) {
                await sleep(500);
                if (await isOllamaUp(host)) return true;
            }
            return false;
        })().finally(() => {
            inflightEnsure = null;
        });
    }
    return inflightEnsure;
}

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

        // 1. 先确保服务在跑（已经在跑就立即返回）
        const ready = await ensureOllamaReady(ollamaHost);
        if (!ready) {
            warn('Ollama did not become reachable after spawn; skipping warmup');
            return;
        }

        info(`Warming up Ollama model: ${ollamaModel} at ${ollamaHost}`);

        // 2. 触发模型加载并钉在内存里（keep_alive: -1）
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
