import type { FetchedTranscript } from './youtube-transcript-api.js';

function secondsToTimestamp(time: number, msSeparator: string = '.'): string {
	const hours = Math.floor(time / 3600);
	const mins = Math.floor((time % 3600) / 60);
	const secs = Math.floor(time % 60);
	const ms = Math.round((time - Math.floor(time)) * 1000);
	function pad(n: number, length: number): string {
		return String(n).padStart(length, '0');
	}

	return `${pad(hours, 2)}:${pad(mins, 2)}:${pad(secs, 2)}${msSeparator}${pad(ms, 3)}`;
}

export function formatJSON(transcript: FetchedTranscript, indent?: number): string {
	return JSON.stringify(transcript.snippets, null, indent);
}

export function formatText(transcript: FetchedTranscript): string {
	return transcript.snippets.map((s) => s.text).join('\n');
}

export function formatSRT(transcript: FetchedTranscript): string {
	const { snippets } = transcript;
	const lines = snippets.map((snippet, i) => {
		const end = snippet.start + snippet.duration;
		const nextStart = i < snippets.length - 1 ? snippets[i + 1].start : end;
		const effectiveEnd = nextStart < end ? nextStart : end;
		const timeText = `${secondsToTimestamp(snippet.start, ',')} --> ${secondsToTimestamp(effectiveEnd, ',')}`;
		return `${i + 1}\n${timeText}\n${snippet.text}`;
	});
	return lines.join('\n\n') + '\n';
}

export function formatWebVTT(transcript: FetchedTranscript): string {
	const { snippets } = transcript;
	const lines = snippets.map((snippet, i) => {
		const end = snippet.start + snippet.duration;
		const nextStart = i < snippets.length - 1 ? snippets[i + 1].start : end;
		const effectiveEnd = nextStart < end ? nextStart : end;
		const timeText = `${secondsToTimestamp(snippet.start, '.')} --> ${secondsToTimestamp(effectiveEnd, '.')}`;
		return `${timeText}\n${snippet.text}`;
	});
	return 'WEBVTT\n\n' + lines.join('\n\n') + '\n';
}
