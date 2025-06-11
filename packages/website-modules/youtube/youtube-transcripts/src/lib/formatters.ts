/* eslint-disable */

/**
 * Type definition for a single transcript line.
 */
export interface TranscriptLine {
	text: string;
	start: number;
	duration: number;
}

/**
 * Abstract base class for formatters.
 */
export abstract class Formatter {
	public abstract formatTranscript(transcript: TranscriptLine[], ...args: any[]): string;

	public abstract formatTranscripts(transcripts: TranscriptLine[][], ...args: any[]): string;
}

/**
 * Pretty print using JSON-like output or custom formatting.
 */
export class PrettyPrintFormatter extends Formatter {
	public formatTranscript(transcript: TranscriptLine[]): string {
		return JSON.stringify(transcript, null, 2);
	}

	public formatTranscripts(transcripts: TranscriptLine[][]): string {
		return JSON.stringify(transcripts, null, 2);
	}
}

/**
 * JSON formatter
 */
export class JSONFormatter extends Formatter {
	public formatTranscript(transcript: TranscriptLine[]): string {
		return JSON.stringify(transcript);
	}

	public formatTranscripts(transcripts: TranscriptLine[][]): string {
		return JSON.stringify(transcripts);
	}
}

/**
 * Basic text-only formatter (omits timestamps)
 */
export class TextFormatter extends Formatter {
	public formatTranscript(transcript: TranscriptLine[]): string {
		return transcript.map((line) => line.text).join('\n');
	}

	public formatTranscripts(transcripts: TranscriptLine[][]): string {
		return transcripts.map(this.formatTranscript).join('\n\n\n');
	}
}

/**
 * SubRip (SRT) format
 */
export class SRTFormatter extends Formatter {
	private secondsToTimeCode(seconds: number): string {
		const ms = Math.round((seconds - Math.floor(seconds)) * 1000);
		const sec = Math.floor(seconds) % 60;
		const min = Math.floor(Math.floor(seconds) / 60) % 60;
		const hr = Math.floor(Math.floor(seconds) / 3600);
		return `${String(hr).padStart(2, '0')}:${String(min).padStart(2, '0')}:${String(
			sec,
		).padStart(2, '0')},${String(ms).padStart(3, '0')}`;
	}

	public formatTranscript(transcript: TranscriptLine[]): string {
		return transcript
			.map((line, i) => {
				const start = this.secondsToTimeCode(line.start);
				const end = this.secondsToTimeCode(line.start + line.duration);
				return `${i + 1}\n${start} --> ${end}\n${line.text}`;
			})
			.join('\n\n');
	}

	public formatTranscripts(transcripts: TranscriptLine[][]): string {
		// Typically you'd do them all combined or separate files.
		// We'll just join them for demonstration.
		return transcripts.map((t) => this.formatTranscript(t)).join('\n\n\n');
	}
}

/**
 * WebVTT format
 */
export class WebVTTFormatter extends Formatter {
	private secondsToTimeCode(seconds: number): string {
		const ms = Math.round((seconds - Math.floor(seconds)) * 1000);
		const sec = Math.floor(seconds) % 60;
		const min = Math.floor(Math.floor(seconds) / 60) % 60;
		const hr = Math.floor(Math.floor(seconds) / 3600);
		return `${String(hr).padStart(2, '0')}:${String(min).padStart(2, '0')}:${String(
			sec,
		).padStart(2, '0')}.${String(ms).padStart(3, '0')}`;
	}

	public formatTranscript(transcript: TranscriptLine[]): string {
		const lines = transcript.map((line, i) => {
			const start = this.secondsToTimeCode(line.start);
			const end = this.secondsToTimeCode(line.start + line.duration);
			return `${start} --> ${end}\n${line.text}`;
		});
		return 'WEBVTT\n\n' + lines.join('\n\n') + '\n';
	}

	public formatTranscripts(transcripts: TranscriptLine[][]): string {
		return transcripts.map((t) => this.formatTranscript(t)).join('\n\n');
	}
}

/**
 * Simple loader akin to the python version
 */
export class FormatterLoader {
	private static TYPES: Record<string, any> = {
		json: JSONFormatter,
		pretty: PrettyPrintFormatter,
		text: TextFormatter,
		webvtt: WebVTTFormatter,
		srt: SRTFormatter,
	};

	public static load(formatterType: string = 'pretty'): Formatter {
		const Ctor = FormatterLoader.TYPES[formatterType];
		if (!Ctor) {
			throw new Error(
				`The format '${formatterType}' is not supported. Choose one of: ${Object.keys(
					FormatterLoader.TYPES,
				).join(', ')}`,
			);
		}
		return new Ctor();
	}
}
