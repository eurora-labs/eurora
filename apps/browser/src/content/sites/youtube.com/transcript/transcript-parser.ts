import he from 'he';
import striptags from 'striptags';

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

export function parseTranscriptXml(
	rawXml: string,
	preserveFormatting: boolean = false,
): TranscriptSnippet[] {
	const allowedTags = preserveFormatting ? FORMATTING_TAGS : [];
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

		const text = he.decode(striptags(innerText, allowedTags));
		const start = parseFloat(el.getAttribute('start')!);
		const duration = parseFloat(el.getAttribute('dur') || '0.0');

		snippets.push({ text, start, duration });
	}

	return snippets;
}
