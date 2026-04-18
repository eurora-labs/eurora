export function middleTruncate(text: string, maxWords = 5): string {
	const parts = text.split(/([^a-zA-Z0-9]+)/);
	const words = parts.filter((p) => /[a-zA-Z0-9]/.test(p));
	if (words.length <= maxWords * 2) return text;

	let start = '';
	let count = 0;
	for (const part of parts) {
		if (/[a-zA-Z0-9]/.test(part)) count++;
		if (count > maxWords) break;
		start += part;
	}

	let end = '';
	count = 0;
	for (let i = parts.length - 1; i >= 0; i--) {
		if (/[a-zA-Z0-9]/.test(parts[i])) count++;
		if (count > maxWords) break;
		end = parts[i] + end;
	}

	return start + '(...)' + end;
}
