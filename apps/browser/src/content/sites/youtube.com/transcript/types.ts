export interface TranscriptSnippet {
	text: string;
	start: number;
	duration: number;
}

export interface FetchedTranscript {
	snippets: TranscriptSnippet[];
	videoId: string;
	language: string;
	languageCode: string;
	isGenerated: boolean;
}

export interface TranslationLanguage {
	language: string;
	languageCode: string;
}

export interface TranscriptInfo {
	videoId: string;
	url: string;
	language: string;
	languageCode: string;
	isGenerated: boolean;
	translationLanguages: TranslationLanguage[];
}

export interface CaptionsJson {
	captionTracks: Array<{
		baseUrl: string;
		name: {
			runs: Array<{ text: string }>;
		};
		languageCode: string;
		kind?: string;
		isTranslatable?: boolean;
	}>;
	translationLanguages?: Array<{
		languageName: {
			runs: Array<{ text: string }>;
		};
		languageCode: string;
	}>;
}

export interface InnertubeData {
	playabilityStatus: {
		status: string;
		reason?: string;
		errorScreen?: {
			playerErrorMessageRenderer?: {
				subreason?: {
					runs?: Array<{ text: string }>;
				};
			};
		};
	};
	captions?: {
		playerCaptionsTracklistRenderer?: CaptionsJson;
	};
}

export class YouTubeTranscriptError extends Error {
	constructor(
		message: string,
		public videoId: string,
	) {
		super(message);
		this.name = 'YouTubeTranscriptError';
	}
}

export class TranscriptsDisabledError extends YouTubeTranscriptError {
	constructor(videoId: string) {
		super('Subtitles are disabled for this video', videoId);
		this.name = 'TranscriptsDisabledError';
	}
}

export class NoTranscriptFoundError extends YouTubeTranscriptError {
	constructor(videoId: string, requestedLanguages: string[]) {
		super(`No transcripts found for languages: ${requestedLanguages.join(', ')}`, videoId);
		this.name = 'NoTranscriptFoundError';
	}
}

export class VideoUnavailableError extends YouTubeTranscriptError {
	constructor(videoId: string) {
		super('The video is no longer available', videoId);
		this.name = 'VideoUnavailableError';
	}
}

export class RequestBlockedError extends YouTubeTranscriptError {
	constructor(videoId: string) {
		super('YouTube is blocking requests', videoId);
		this.name = 'RequestBlockedError';
	}
}
