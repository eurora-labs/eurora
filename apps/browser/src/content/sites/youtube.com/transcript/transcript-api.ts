import {
	YouTubeTranscriptError,
	TranscriptsDisabledError,
	NoTranscriptFoundError,
	VideoUnavailableError,
	RequestBlockedError,
} from './types.js';
import type {
	TranscriptSnippet,
	FetchedTranscript,
	TranslationLanguage,
	CaptionsJson,
	InnertubeData,
} from './types.js';

export class Transcript {
	constructor(
		private videoId: string,
		private url: string,
		public language: string,
		public languageCode: string,
		public isGenerated: boolean,
		public translationLanguages: TranslationLanguage[],
	) {}

	async fetch(preserveFormatting: boolean = false): Promise<FetchedTranscript> {
		try {
			const response = await fetch(this.url);
			if (!response.ok) {
				throw new YouTubeTranscriptError(
					`HTTP ${response.status}: ${response.statusText}`,
					this.videoId,
				);
			}

			const xmlText = await response.text();
			const snippets = this.parseTranscriptXml(xmlText, preserveFormatting);

			return {
				snippets,
				videoId: this.videoId,
				language: this.language,
				languageCode: this.languageCode,
				isGenerated: this.isGenerated,
			};
		} catch (error) {
			if (error instanceof YouTubeTranscriptError) {
				throw error;
			}
			throw new YouTubeTranscriptError(`Failed to fetch transcript: ${error}`, this.videoId);
		}
	}

	private parseTranscriptXml(xmlText: string, preserveFormatting: boolean): TranscriptSnippet[] {
		const parser = new DOMParser();
		const xmlDoc = parser.parseFromString(xmlText, 'text/xml');
		const textElements = xmlDoc.querySelectorAll('text');

		const snippets: TranscriptSnippet[] = [];

		textElements.forEach((element) => {
			const text = element.textContent;
			const start = parseFloat(element.getAttribute('start') || '0');
			const duration = parseFloat(element.getAttribute('dur') || '0');

			if (text) {
				let cleanText = this.unescapeHtml(text);
				if (!preserveFormatting) {
					cleanText = cleanText.replace(/<[^>]*>/g, '');
				}

				snippets.push({
					text: cleanText,
					start,
					duration,
				});
			}
		});

		return snippets;
	}

	private unescapeHtml(text: string): string {
		const div = document.createElement('div');
		div.innerHTML = text;
		return div.textContent || div.innerText || '';
	}

	get isTranslatable(): boolean {
		return this.translationLanguages.length > 0;
	}

	translate(languageCode: string): Transcript {
		if (!this.isTranslatable) {
			throw new YouTubeTranscriptError('This transcript is not translatable', this.videoId);
		}

		const targetLanguage = this.translationLanguages.find(
			(lang) => lang.languageCode === languageCode,
		);
		if (!targetLanguage) {
			throw new YouTubeTranscriptError(
				`Translation language ${languageCode} not available`,
				this.videoId,
			);
		}

		const translatedUrl = `${this.url}&tlang=${languageCode}`;
		return new Transcript(
			this.videoId,
			translatedUrl,
			targetLanguage.language,
			languageCode,
			true,
			[],
		);
	}
}

export class TranscriptList {
	constructor(
		public videoId: string,
		private manuallyCreatedTranscripts: Map<string, Transcript>,
		private generatedTranscripts: Map<string, Transcript>,
		private translationLanguages: TranslationLanguage[],
	) {}

	static async build(videoId: string, captionsJson: CaptionsJson): Promise<TranscriptList> {
		const translationLanguages: TranslationLanguage[] =
			captionsJson.translationLanguages?.map((lang) => ({
				language: lang.languageName.runs[0].text,
				languageCode: lang.languageCode,
			})) || [];

		const manuallyCreatedTranscripts = new Map<string, Transcript>();
		const generatedTranscripts = new Map<string, Transcript>();

		for (const caption of captionsJson.captionTracks) {
			const isGenerated = caption.kind === 'asr';
			const targetMap = isGenerated ? generatedTranscripts : manuallyCreatedTranscripts;

			const transcript = new Transcript(
				videoId,
				caption.baseUrl.replace('&fmt=srv3', ''),
				caption.name.runs[0].text,
				caption.languageCode,
				isGenerated,
				caption.isTranslatable ? translationLanguages : [],
			);

			targetMap.set(caption.languageCode, transcript);
		}

		return new TranscriptList(
			videoId,
			manuallyCreatedTranscripts,
			generatedTranscripts,
			translationLanguages,
		);
	}

	findTranscript(languageCodes: string[]): Transcript {
		return this.findTranscriptInternal(languageCodes, [
			this.manuallyCreatedTranscripts,
			this.generatedTranscripts,
		]);
	}

	findGeneratedTranscript(languageCodes: string[]): Transcript {
		return this.findTranscriptInternal(languageCodes, [this.generatedTranscripts]);
	}

	findManuallyCreatedTranscript(languageCodes: string[]): Transcript {
		return this.findTranscriptInternal(languageCodes, [this.manuallyCreatedTranscripts]);
	}

	private findTranscriptInternal(
		languageCodes: string[],
		transcriptMaps: Map<string, Transcript>[],
	): Transcript {
		for (const languageCode of languageCodes) {
			for (const transcriptMap of transcriptMaps) {
				const transcript = transcriptMap.get(languageCode);
				if (transcript) {
					return transcript;
				}
			}
		}

		throw new NoTranscriptFoundError(this.videoId, languageCodes);
	}

	*[Symbol.iterator](): Iterator<Transcript> {
		yield* this.manuallyCreatedTranscripts.values();
		yield* this.generatedTranscripts.values();
	}

	getAllTranscripts(): Transcript[] {
		return [...this];
	}
}

export class YouTubeTranscriptApi {
	async fetch(
		videoId: string,
		languages: string[] = ['en'],
		preserveFormatting: boolean = false,
	): Promise<FetchedTranscript> {
		const transcriptList = await this.list(videoId);
		const transcript = transcriptList.findTranscript(languages);
		return await transcript.fetch(preserveFormatting);
	}

	async list(videoId: string): Promise<TranscriptList> {
		try {
			const captionsJson = await this.fetchCaptionsJson(videoId);
			return await TranscriptList.build(videoId, captionsJson);
		} catch (error) {
			if (error instanceof YouTubeTranscriptError) {
				throw error;
			}
			throw new YouTubeTranscriptError(`Failed to list transcripts: ${error}`, videoId);
		}
	}

	private async fetchCaptionsJson(videoId: string): Promise<CaptionsJson> {
		const html = document.documentElement.outerHTML;
		const apiKey = this.extractInnertubeApiKey(html, videoId);
		const innertubeData = await this.fetchInnertubeData(videoId, apiKey);
		return this.extractCaptionsJson(innertubeData, videoId);
	}

	private extractInnertubeApiKey(html: string, videoId: string): string {
		const pattern = /"INNERTUBE_API_KEY":\s*"([a-zA-Z0-9_-]+)"/;
		const match = html.match(pattern);

		if (match && match[1]) {
			return match[1];
		}

		if (html.includes('class="g-recaptcha"')) {
			throw new RequestBlockedError(videoId);
		}

		throw new YouTubeTranscriptError('Could not extract API key from page', videoId);
	}

	private async fetchInnertubeData(videoId: string, apiKey: string): Promise<InnertubeData> {
		const url = `https://www.youtube.com/youtubei/v1/player?key=${apiKey}`;
		const payload = {
			context: {
				client: {
					clientName: 'ANDROID',
					clientVersion: '20.10.38',
				},
			},
			videoId: videoId,
		};

		try {
			const response = await fetch(url, {
				method: 'POST',
				headers: {
					'Content-Type': 'application/json',
				},
				body: JSON.stringify(payload),
			});

			if (!response.ok) {
				throw new YouTubeTranscriptError(
					`HTTP ${response.status}: ${response.statusText}`,
					videoId,
				);
			}

			return await response.json();
		} catch (error) {
			if (error instanceof YouTubeTranscriptError) {
				throw error;
			}
			throw new YouTubeTranscriptError(`Failed to fetch innertube data: ${error}`, videoId);
		}
	}

	private extractCaptionsJson(innertubeData: InnertubeData, videoId: string): CaptionsJson {
		this.assertPlayability(innertubeData.playabilityStatus, videoId);

		const captionsJson = innertubeData.captions?.playerCaptionsTracklistRenderer;
		if (!captionsJson || !captionsJson.captionTracks) {
			throw new TranscriptsDisabledError(videoId);
		}

		return captionsJson;
	}

	private assertPlayability(playabilityStatus: any, videoId: string): void {
		if (playabilityStatus.status !== 'OK') {
			const reason = playabilityStatus.reason;

			if (playabilityStatus.status === 'LOGIN_REQUIRED') {
				if (reason === "Sign in to confirm you're not a bot") {
					throw new RequestBlockedError(videoId);
				}
			}

			if (playabilityStatus.status === 'ERROR' && reason === 'This video is unavailable') {
				if (videoId.startsWith('http://') || videoId.startsWith('https://')) {
					throw new YouTubeTranscriptError(
						'Invalid video ID - use video ID, not URL',
						videoId,
					);
				}
				throw new VideoUnavailableError(videoId);
			}

			throw new YouTubeTranscriptError(
				`Video unplayable: ${reason || 'Unknown reason'}`,
				videoId,
			);
		}
	}
}
