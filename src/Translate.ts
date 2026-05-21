const translationCache = new Map<string, string>()

type GoogleTranslateResponse = [
    [string, string, unknown, unknown?][],
    unknown,
    string
]

export async function translateLyrics(
    text: string,
    targetLanguage: string
): Promise<string> {
    if (!text.trim()) return text

    const cacheKey = `${targetLanguage}_${text}`

    const cached = translationCache.get(cacheKey)
    if (cached) return cached

    const url =
        "https://translate.googleapis.com/translate_a/single" +
        `?client=gtx&sl=auto&tl=${targetLanguage}&dt=t&q=${encodeURIComponent(text)}`

    try {
        const response = await fetch(url, {
            cache: "force-cache",
        })

        if (!response.ok) {
            throw new Error(`HTTP ${response.status}`)
        }

        const data = (await response.json()) as GoogleTranslateResponse

        if (!Array.isArray(data) || !Array.isArray(data[0])) {
            throw new Error("Unexpected translation response")
        }

        const translatedText = data[0]
            .map((part) => part[0] || "")
            .join("")
            .trim()

        // Evita guardar traducciones idénticas
        if (
            !translatedText ||
            translatedText.toLowerCase() === text.trim().toLowerCase()
        ) {
            return text
        }

        translationCache.set(cacheKey, translatedText)

        return translatedText
    } catch (error) {
        console.warn(
            "[Translation] Error translating lyrics:",
            error instanceof Error ? error.message : error
        )

        return text
    }
}

export function clearTranslationCache(): void {
    translationCache.clear()
}
