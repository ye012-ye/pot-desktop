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

    const loadAttempts = options.loadAttempts ?? 3;
    for (let attempt = 1; attempt <= loadAttempts; attempt++) {
        try {
            await dependencies.loadModel(entry, target);
            break;
        } catch (error) {
            if (attempt === loadAttempts) throw error;
            await dependencies.sleep(intervalMs);
        }
    }
    return { status: 'loaded', entry, target };
}
