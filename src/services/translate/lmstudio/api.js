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
