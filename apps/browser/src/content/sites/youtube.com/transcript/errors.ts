const WATCH_URL = 'https://www.youtube.com/watch?v={video_id}';

export class YouTubeTranscriptApiError extends Error {
	constructor(message: string) {
		super(message);
		this.name = this.constructor.name;
	}
}

export class CouldNotRetrieveTranscript extends YouTubeTranscriptApiError {
	static CAUSE_MESSAGE = '';

	public readonly videoId: string;

	constructor(videoId: string) {
		super('');
		this.videoId = videoId;
		this.message = this._buildErrorMessage();
	}

	get cause(): string {
		return (this.constructor as typeof CouldNotRetrieveTranscript).CAUSE_MESSAGE;
	}

	protected _buildErrorMessage(): string {
		const videoUrl = WATCH_URL.replace('{video_id}', this.videoId);
		let msg = `\nCould not retrieve a transcript for the video ${videoUrl}!`;
		const cause = this.cause;
		if (cause) {
			msg += ` This is most likely caused by:\n\n${cause}`;
		}
		return msg;
	}
}

export class YouTubeDataUnparsable extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE = 'The data required to fetch the transcript is not parsable.';
}

export class YouTubeRequestFailed extends CouldNotRetrieveTranscript {
	public readonly statusCode: number;
	public readonly statusText: string;

	constructor(videoId: string, statusCode: number, statusText: string) {
		super(videoId);
		this.statusCode = statusCode;
		this.statusText = statusText;
		this.message = this._buildErrorMessage();
	}

	override get cause(): string {
		return `Request to YouTube failed: ${this.statusCode} ${this.statusText}`;
	}
}

export class VideoUnplayable extends CouldNotRetrieveTranscript {
	public readonly reason: string | null;
	public readonly subReasons: string[];

	constructor(videoId: string, reason: string | null, subReasons: string[] = []) {
		super(videoId);
		this.reason = reason;
		this.subReasons = subReasons;
		this.message = this._buildErrorMessage();
	}

	override get cause(): string {
		let reason = this.reason ?? 'No reason specified!';
		if (this.subReasons.length > 0) {
			const subs = this.subReasons.map((r) => ` - ${r}`).join('\n');
			reason += `\n\nAdditional Details:\n${subs}`;
		}
		return `The video is unplayable for the following reason: ${reason}`;
	}
}

export class VideoUnavailable extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE = 'The video is no longer available';
}

export class InvalidVideoId extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE =
		'You provided an invalid video id. Make sure you are using the video id and NOT the url!\n\n' +
		'Do NOT run: YouTubeTranscriptApi.fetch("https://www.youtube.com/watch?v=1234")\n' +
		'Instead run: YouTubeTranscriptApi.fetch("1234")';
}

export class RequestBlocked extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE = 'YouTube is blocking requests from your IP.';
}

export class IpBlocked extends RequestBlocked {
	static override CAUSE_MESSAGE =
		'YouTube is blocking requests from your IP. A reCAPTCHA was detected.';
}

export class TranscriptsDisabled extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE = 'Subtitles are disabled for this video';
}

export class AgeRestricted extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE =
		'This video is age-restricted. Transcripts cannot be retrieved without authentication.';
}

export class NotTranslatable extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE = 'The requested language is not translatable';
}

export class TranslationLanguageNotAvailable extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE = 'The requested translation language is not available';
}

export class PoTokenRequired extends CouldNotRetrieveTranscript {
	static override CAUSE_MESSAGE = 'The requested video cannot be retrieved without a PO Token.';
}

export class NoTranscriptFound extends CouldNotRetrieveTranscript {
	public readonly requestedLanguageCodes: string[];
	public readonly transcriptList: unknown;

	constructor(videoId: string, requestedLanguageCodes: string[], transcriptList: unknown) {
		super(videoId);
		this.requestedLanguageCodes = requestedLanguageCodes;
		this.transcriptList = transcriptList;
		this.message = this._buildErrorMessage();
	}

	override get cause(): string {
		return (
			`No transcripts were found for any of the requested language codes: ${JSON.stringify(this.requestedLanguageCodes)}\n\n` +
			`${this.transcriptList}`
		);
	}
}
