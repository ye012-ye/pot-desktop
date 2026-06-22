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
