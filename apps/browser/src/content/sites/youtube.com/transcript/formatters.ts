import { stringifySync, type NodeList } from 'subtitle';
import type { FetchedTranscript } from './youtube-transcript-api.js';

function buildCues(transcript: FetchedTranscript): NodeList {
	const { snippets } = transcript;
	return snippets.map((snippet, i) => {
		const end = snippet.start + snippet.duration;
		const nextStart = i < snippets.length - 1 ? snippets[i + 1].start : end;
		const effectiveEnd = nextStart < end ? nextStart : end;
		return {
			type: 'cue' as const,
			data: {
				start: Math.round(snippet.start * 1000),
				end: Math.round(effectiveEnd * 1000),
				text: snippet.text,
			},
		};
	});
}

export function formatJSON(transcript: FetchedTranscript, indent?: number): string {
	return JSON.stringify(transcript.snippets, null, indent);
}

export function formatText(transcript: FetchedTranscript): string {
	return transcript.snippets.map((s) => s.text).join('\n');
}

export function formatSRT(transcript: FetchedTranscript): string {
	return stringifySync(buildCues(transcript), { format: 'SRT' });
}

export function formatWebVTT(transcript: FetchedTranscript): string {
	return stringifySync(buildCues(transcript), { format: 'WebVTT' });
}
