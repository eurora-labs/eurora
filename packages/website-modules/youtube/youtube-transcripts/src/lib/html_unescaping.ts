/**
 * Simple HTML unescape function.
 * In modern browsers, we can rely on the standard built-in APIs
 * or do manual replacements.
 * For example, we can do:
 *   new DOMParser().parseFromString(str, "text/html").documentElement.textContent
 */

export function unescape(str: string): string {
    // Quick version using a temporary textarea
    const textarea = document.createElement('textarea');
    textarea.innerHTML = str;
    return textarea.value;
}
