export interface TranscriptSnippet {
	text: string;
	start: number;
	duration: number;
}

const FORMATTING_TAGS: string[] = [
	'strong',
	'em',
	'b',
	'i',
	'mark',
	'small',
	'del',
	'ins',
	'sub',
	'sup',
];

function getHtmlRegex(preserveFormatting: boolean): RegExp {
	if (preserveFormatting) {
		const formatsRegex = FORMATTING_TAGS.join('|');
		return new RegExp(`<\\/?(?!\\/?(?:${formatsRegex})\\b).*?\\b>`, 'gi');
	}
	return /<[^>]*>/gi;
}

function htmlUnescape(text: string): string {
	const doc = new DOMParser().parseFromString(text, 'text/html');
	return doc.documentElement.textContent ?? '';
}

export function parseTranscriptXml(
	rawXml: string,
	preserveFormatting: boolean = false,
): TranscriptSnippet[] {
	const htmlRegex = getHtmlRegex(preserveFormatting);
	const parser = new DOMParser();
	const doc = parser.parseFromString(rawXml, 'text/xml');
	const elements = doc.querySelectorAll('text');
	const snippets: TranscriptSnippet[] = [];

	for (const el of Array.from(elements)) {
		const serializer = new XMLSerializer();
		let innerText = '';
		for (const child of Array.from(el.childNodes)) {
			if (child.nodeType === Node.TEXT_NODE) {
				innerText += child.textContent;
			} else {
				innerText += serializer.serializeToString(child);
			}
		}

		if (!innerText) continue;

		const text = htmlUnescape(innerText.replace(htmlRegex, ''));
		const start = parseFloat(el.getAttribute('start')!);
		const duration = parseFloat(el.getAttribute('dur') || '0.0');

		snippets.push({ text, start, duration });
	}

	return snippets;
}
