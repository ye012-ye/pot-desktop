const MAX_INLINE_TRANSLATE_CACHE_SIZE = 200;

const inlineTranslateCache = new Map();

function normalizeText(text) {
    return typeof text === 'string' ? text.trim() : '';
}

function remember(source, target) {
    inlineTranslateCache.delete(source);
    inlineTranslateCache.set(source, target);

    if (inlineTranslateCache.size > MAX_INLINE_TRANSLATE_CACHE_SIZE) {
        inlineTranslateCache.delete(inlineTranslateCache.keys().next().value);
    }
}

export function cacheInlineTranslatePair(sourceText, targetText) {
    const source = normalizeText(sourceText);
    const target = normalizeText(targetText);

    if (!source || !target || source === target) {
        return;
    }

    remember(source, target);
    remember(target, source);
}

export function getCachedInlineTranslate(text) {
    return inlineTranslateCache.get(normalizeText(text)) ?? null;
}

export function clearInlineTranslateCache() {
    inlineTranslateCache.clear();
}
