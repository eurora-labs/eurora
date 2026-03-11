import {
	VideoUnavailable,
	VideoUnplayable,
	InvalidVideoId,
	TranscriptsDisabled,
	AgeRestricted,
	RequestBlocked,
	IpBlocked,
	YouTubeRequestFailed,
	YouTubeDataUnparsable,
	NotTranslatable,
	TranslationLanguageNotAvailable,
	NoTranscriptFound,
	PoTokenRequired,
} from './errors.js';
import { parseTranscriptXml, type TranscriptSnippet } from './transcript-parser.js';

export type { TranscriptSnippet };

const WATCH_URL = 'https://www.youtube.com/watch?v={video_id}';
const INNERTUBE_API_URL = 'https://www.youtube.com/youtubei/v1/player?key={api_key}';
const INNERTUBE_CONTEXT = {
	client: { clientName: 'ANDROID', clientVersion: '20.10.38' },
};

interface InnertubePlayabilityStatus {
	status?: string;
	reason?: string;
	errorScreen?: {
		playerErrorMessageRenderer?: {
			subreason?: {
				runs?: Array<{ text?: string }>;
			};
		};
	};
}

interface InnertubeCaptionTrack {
	baseUrl: string;
	name: { runs: Array<{ text: string }> };
	languageCode: string;
	kind?: string;
	isTranslatable?: boolean;
}

interface InnertubeTranslationLanguage {
	languageCode: string;
	languageName: { runs: Array<{ text: string }> };
}

interface InnertubeCaptionsJson {
	captionTracks: InnertubeCaptionTrack[];
	translationLanguages?: InnertubeTranslationLanguage[];
}

interface InnertubeResponse {
	playabilityStatus?: InnertubePlayabilityStatus;
	captions?: {
		playerCaptionsTracklistRenderer?: InnertubeCaptionsJson;
	};
}

export interface TranslationLanguage {
	language: string;
	languageCode: string;
}

export interface FetchOptions {
	languages?: string[];
	preserveFormatting?: boolean;
}

export class FetchedTranscript {
	public readonly snippets: TranscriptSnippet[];
	public readonly videoId: string;
	public readonly language: string;
	public readonly languageCode: string;
	public readonly isGenerated: boolean;

	constructor(
		snippets: TranscriptSnippet[],
		videoId: string,
		language: string,
		languageCode: string,
		isGenerated: boolean,
	) {
		this.snippets = snippets;
		this.videoId = videoId;
		this.language = language;
		this.languageCode = languageCode;
		this.isGenerated = isGenerated;
	}

	[Symbol.iterator](): Iterator<TranscriptSnippet> {
		return this.snippets[Symbol.iterator]();
	}

	get length(): number {
		return this.snippets.length;
	}

	toRawData(): TranscriptSnippet[] {
		return this.snippets.map((s) => ({ ...s }));
	}
}

export class Transcript {
	public readonly videoId: string;
	public readonly language: string;
	public readonly languageCode: string;
	public readonly isGenerated: boolean;
	public readonly translationLanguages: TranslationLanguage[];

	private readonly _url: string;
	private readonly _translationLanguagesDict: Record<string, string>;

	constructor(
		videoId: string,
		url: string,
		language: string,
		languageCode: string,
		isGenerated: boolean,
		translationLanguages: TranslationLanguage[],
	) {
		this.videoId = videoId;
		this._url = url;
		this.language = language;
		this.languageCode = languageCode;
		this.isGenerated = isGenerated;
		this.translationLanguages = translationLanguages;
		this._translationLanguagesDict = Object.fromEntries(
			translationLanguages.map((tl) => [tl.languageCode, tl.language]),
		);
	}

	get isTranslatable(): boolean {
		return this.translationLanguages.length > 0;
	}

	async fetch(preserveFormatting: boolean = false): Promise<FetchedTranscript> {
		if (this._url.includes('&exp=xpe')) {
			throw new PoTokenRequired(this.videoId);
		}

		const response = await globalThis.fetch(this._url);
		if (response.status === 429) throw new IpBlocked(this.videoId);
		if (!response.ok) {
			throw new YouTubeRequestFailed(this.videoId, response.status, response.statusText);
		}

		const rawXml = await response.text();
		const snippets = parseTranscriptXml(rawXml, preserveFormatting);

		return new FetchedTranscript(
			snippets,
			this.videoId,
			this.language,
			this.languageCode,
			this.isGenerated,
		);
	}

	translate(languageCode: string): Transcript {
		if (!this.isTranslatable) {
			throw new NotTranslatable(this.videoId);
		}
		if (!(languageCode in this._translationLanguagesDict)) {
			throw new TranslationLanguageNotAvailable(this.videoId);
		}
		return new Transcript(
			this.videoId,
			`${this._url}&tlang=${languageCode}`,
			this._translationLanguagesDict[languageCode],
			languageCode,
			true,
			[],
		);
	}

	toString(): string {
		const translatable = this.isTranslatable ? '[TRANSLATABLE]' : '';
		return `${this.languageCode} ("${this.language}")${translatable}`;
	}
}

export class TranscriptList {
	public readonly videoId: string;

	private readonly _manuallyCreated: Record<string, Transcript>;
	private readonly _generated: Record<string, Transcript>;
	private readonly _translationLanguages: TranslationLanguage[];

	constructor(
		videoId: string,
		manuallyCreated: Record<string, Transcript>,
		generated: Record<string, Transcript>,
		translationLanguages: TranslationLanguage[],
	) {
		this.videoId = videoId;
		this._manuallyCreated = manuallyCreated;
		this._generated = generated;
		this._translationLanguages = translationLanguages;
	}

	static build(videoId: string, captionsJson: InnertubeCaptionsJson): TranscriptList {
		const translationLanguages: TranslationLanguage[] = (
			captionsJson.translationLanguages || []
		).map((tl) => ({
			language: tl.languageName.runs[0].text,
			languageCode: tl.languageCode,
		}));

		const manuallyCreated: Record<string, Transcript> = {};
		const generated: Record<string, Transcript> = {};

		for (const caption of captionsJson.captionTracks) {
			const isAsr = (caption.kind || '') === 'asr';
			const dict = isAsr ? generated : manuallyCreated;
			const url = caption.baseUrl.replace('&fmt=srv3', '');

			dict[caption.languageCode] = new Transcript(
				videoId,
				url,
				caption.name.runs[0].text,
				caption.languageCode,
				isAsr,
				caption.isTranslatable ? translationLanguages : [],
			);
		}

		return new TranscriptList(videoId, manuallyCreated, generated, translationLanguages);
	}

	[Symbol.iterator](): Iterator<Transcript> {
		const all = [...Object.values(this._manuallyCreated), ...Object.values(this._generated)];
		return all[Symbol.iterator]();
	}

	findTranscript(languageCodes: string[]): Transcript {
		return this._findTranscript(languageCodes, [this._manuallyCreated, this._generated]);
	}

	findGeneratedTranscript(languageCodes: string[]): Transcript {
		return this._findTranscript(languageCodes, [this._generated]);
	}

	findManuallyCreatedTranscript(languageCodes: string[]): Transcript {
		return this._findTranscript(languageCodes, [this._manuallyCreated]);
	}

	private _findTranscript(
		languageCodes: string[],
		dicts: Record<string, Transcript>[],
	): Transcript {
		for (const code of languageCodes) {
			for (const dict of dicts) {
				if (code in dict) return dict[code];
			}
		}
		throw new NoTranscriptFound(this.videoId, languageCodes, this);
	}

	toString(): string {
		function fmt(dict: Record<string, Transcript>): string {
			const entries = Object.values(dict)
				.map((t) => ` - ${t}`)
				.join('\n');
			return entries || 'None';
		}
		const tlFmt =
			this._translationLanguages
				.map((tl) => ` - ${tl.languageCode} ("${tl.language}")`)
				.join('\n') || 'None';
		return (
			`For this video (${this.videoId}) transcripts are available in the following languages:\n\n` +
			`(MANUALLY CREATED)\n${fmt(this._manuallyCreated)}\n\n` +
			`(GENERATED)\n${fmt(this._generated)}\n\n` +
			`(TRANSLATION LANGUAGES)\n${tlFmt}`
		);
	}
}

const PlayabilityStatus = {
	OK: 'OK',
	ERROR: 'ERROR',
	LOGIN_REQUIRED: 'LOGIN_REQUIRED',
} as const;

const PlayabilityFailedReason = {
	BOT_DETECTED: "Sign in to confirm you're not a bot",
	AGE_RESTRICTED: 'This video may be inappropriate for some users.',
	VIDEO_UNAVAILABLE: 'This video is unavailable',
} as const;

function assertPlayability(status: InnertubePlayabilityStatus | undefined, videoId: string): void {
	const playabilityStatus = status?.status;
	if (playabilityStatus === PlayabilityStatus.OK || playabilityStatus === null) {
		return;
	}

	const reason = status!.reason;

	if (playabilityStatus === PlayabilityStatus.LOGIN_REQUIRED) {
		if (reason === PlayabilityFailedReason.BOT_DETECTED) {
			throw new RequestBlocked(videoId);
		}
		if (reason === PlayabilityFailedReason.AGE_RESTRICTED) {
			throw new AgeRestricted(videoId);
		}
	}

	if (
		playabilityStatus === PlayabilityStatus.ERROR &&
		reason === PlayabilityFailedReason.VIDEO_UNAVAILABLE
	) {
		if (videoId.startsWith('http://') || videoId.startsWith('https://')) {
			throw new InvalidVideoId(videoId);
		}
		throw new VideoUnavailable(videoId);
	}

	const subReasons = (status!.errorScreen?.playerErrorMessageRenderer?.subreason?.runs || []).map(
		(r) => r.text || '',
	);

	throw new VideoUnplayable(videoId, reason ?? null, subReasons);
}

export class YouTubeTranscriptApi {
	async fetch(
		videoId: string,
		{ languages = ['en'], preserveFormatting = false }: FetchOptions = {},
	): Promise<FetchedTranscript> {
		const list = await this.list(videoId);
		return await list.findTranscript(languages).fetch(preserveFormatting);
	}

	async list(videoId: string): Promise<TranscriptList> {
		if (videoId.startsWith('http://') || videoId.startsWith('https://')) {
			throw new InvalidVideoId(videoId);
		}
		const captionsJson = await this._fetchCaptionsJson(videoId);
		return TranscriptList.build(videoId, captionsJson);
	}

	private async _fetchCaptionsJson(videoId: string): Promise<InnertubeCaptionsJson> {
		const html = await this._fetchHtml(videoId);
		const apiKey = this._extractInnertubeApiKey(html, videoId);
		const innertubeData = await this._fetchInnertubeData(videoId, apiKey);
		return this._extractCaptionsJson(innertubeData, videoId);
	}

	private _extractInnertubeApiKey(html: string, videoId: string): string {
		const match = html.match(/"INNERTUBE_API_KEY":\s*"([a-zA-Z0-9_-]+)"/);
		if (match && match[1]) return match[1];
		if (html.includes('class="g-recaptcha"')) throw new IpBlocked(videoId);
		throw new YouTubeDataUnparsable(videoId);
	}

	private _extractCaptionsJson(
		innertubeData: InnertubeResponse,
		videoId: string,
	): InnertubeCaptionsJson {
		assertPlayability(innertubeData.playabilityStatus, videoId);

		const captionsJson = innertubeData.captions?.playerCaptionsTracklistRenderer;
		if (!captionsJson || !captionsJson.captionTracks) {
			throw new TranscriptsDisabled(videoId);
		}
		return captionsJson;
	}

	private async _fetchHtml(videoId: string): Promise<string> {
		const url = WATCH_URL.replace('{video_id}', videoId);
		const response = await globalThis.fetch(url, {
			headers: { 'Accept-Language': 'en-US' },
		});
		if (response.status === 429) throw new IpBlocked(videoId);
		if (!response.ok) {
			throw new YouTubeRequestFailed(videoId, response.status, response.statusText);
		}
		return await response.text();
	}

	private async _fetchInnertubeData(videoId: string, apiKey: string): Promise<InnertubeResponse> {
		const url = INNERTUBE_API_URL.replace('{api_key}', apiKey);
		const response = await globalThis.fetch(url, {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json',
				'Accept-Language': 'en-US',
			},
			body: JSON.stringify({
				context: INNERTUBE_CONTEXT,
				videoId,
			}),
		});
		if (response.status === 429) throw new IpBlocked(videoId);
		if (!response.ok) {
			throw new YouTubeRequestFailed(videoId, response.status, response.statusText);
		}
		return await response.json();
	}
}

export default YouTubeTranscriptApi;
