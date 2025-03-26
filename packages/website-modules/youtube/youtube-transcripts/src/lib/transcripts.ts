/* eslint-disable */

import {
    VideoUnavailable,
    TooManyRequests,
    YouTubeRequestFailed,
    NoTranscriptFound,
    TranscriptsDisabled,
    NotTranslatable,
    TranslationLanguageNotAvailable,
    NoTranscriptAvailable,
    FailedToCreateConsentCookie,
    InvalidVideoId,
} from './errors.js';
import { unescape } from './html_unescaping.js';
import { WATCH_URL } from './settings.js';

/**
 * Helper to transform fetch's Response into an error if not ok.
 */
async function raiseHttpErrors(
    response: Response,
    videoId: string,
): Promise<Response> {
    if (!response.ok) {
        const text = await response.text();
        // Throw a more specialized error
        throw new YouTubeRequestFailed(
            videoId,
            `Status: ${response.status}, Body: ${text}`,
        );
    }
    return response;
}

export type TranscriptLine = {
    text: string;
    start: number;
    duration: number;
};

export class TranscriptListFetcher {
    private _httpClient: {
        get(
            url: string,
            extraHeaders?: Record<string, string>,
        ): Promise<Response>;
    };

    constructor(httpClient: {
        get(
            url: string,
            extraHeaders?: Record<string, string>,
        ): Promise<Response>;
    }) {
        this._httpClient = httpClient;
    }

    public async fetch(videoId: string): Promise<TranscriptList> {
        const html = await this._fetchVideoHtml(videoId);
        const captionsJson = this._extractCaptionsJson(html, videoId);

        return TranscriptList.build(this._httpClient, videoId, captionsJson);
    }

    private _extractCaptionsJson(html: string, videoId: string): any {
        const splittedHtml = html.split('"captions":');

        if (splittedHtml.length <= 1) {
            if (
                videoId.startsWith('http://') ||
                videoId.startsWith('https://')
            ) {
                throw new InvalidVideoId(videoId);
            }
            if (html.includes('class="g-recaptcha"')) {
                throw new TooManyRequests(videoId);
            }
            if (!html.includes('"playabilityStatus":')) {
                throw new VideoUnavailable(videoId);
            }
            throw new TranscriptsDisabled(videoId);
        }

        const splittedFurther = splittedHtml[1].split(',"videoDetails');
        if (splittedFurther.length === 0) {
            throw new TranscriptsDisabled(videoId);
        }

        const jsonStr = splittedFurther[0].replace(/\n/g, '');
        const parsed = JSON.parse(jsonStr).playerCaptionsTracklistRenderer;
        if (!parsed) {
            throw new TranscriptsDisabled(videoId);
        }

        if (!('captionTracks' in parsed)) {
            throw new NoTranscriptAvailable(videoId);
        }

        return parsed;
    }

    private _createConsentCookie(html: string, videoId: string) {
        const match = /name="v" value="(.*?)"/.exec(html);
        if (match === null) {
            throw new FailedToCreateConsentCookie(videoId);
        }
        // In the browser, you'd typically set document.cookie or rely on fetch auto-handling
        document.cookie = `CONSENT=YES+${match[1]}; domain=.youtube.com`;
    }

    private async _fetchVideoHtml(videoId: string): Promise<string> {
        let html = await this._fetchHtml(videoId);
        if (html.includes('action="https://consent.youtube.com/s"')) {
            this._createConsentCookie(html, videoId);
            html = await this._fetchHtml(videoId);
            if (html.includes('action="https://consent.youtube.com/s"')) {
                throw new FailedToCreateConsentCookie(videoId);
            }
        }
        return html;
    }

    private async _fetchHtml(videoId: string): Promise<string> {
        const url = WATCH_URL.replace(
            '{video_id}',
            encodeURIComponent(videoId),
        );
        const response = await this._httpClient.get(url);
        await raiseHttpErrors(response, videoId);
        const text = await response.text();
        return unescape(text);
    }
}

export class TranscriptList {
    public videoId: string;
    private _manuallyCreatedTranscripts: Record<string, Transcript>;
    private _generatedTranscripts: Record<string, Transcript>;
    private _translationLanguages: Array<{
        language: string;
        language_code: string;
    }>;

    constructor(
        videoId: string,
        manuallyCreatedTranscripts: Record<string, Transcript>,
        generatedTranscripts: Record<string, Transcript>,
        translationLanguages: Array<{
            language: string;
            language_code: string;
        }>,
    ) {
        this.videoId = videoId;
        this._manuallyCreatedTranscripts = manuallyCreatedTranscripts;
        this._generatedTranscripts = generatedTranscripts;
        this._translationLanguages = translationLanguages;
    }

    public static build(
        httpClient: {
            get(
                url: string,
                extraHeaders?: Record<string, string>,
            ): Promise<Response>;
        },
        videoId: string,
        captionsJson: any,
    ): TranscriptList {
        const translationLanguages = (
            captionsJson.translationLanguages ?? []
        ).map((langObj: any) => ({
            language: langObj.languageName.simpleText,
            language_code: langObj.languageCode,
        }));

        const manuallyCreatedTranscripts: Record<string, Transcript> = {};
        const generatedTranscripts: Record<string, Transcript> = {};

        for (const caption of captionsJson.captionTracks) {
            const isAsr = caption.kind === 'asr';
            const transcriptMap = isAsr
                ? generatedTranscripts
                : manuallyCreatedTranscripts;
            transcriptMap[caption.languageCode] = new Transcript(
                httpClient,
                videoId,
                caption.baseUrl,
                caption.name.simpleText,
                caption.languageCode,
                isAsr,
                caption.isTranslatable ? translationLanguages : [],
            );
        }

        return new TranscriptList(
            videoId,
            manuallyCreatedTranscripts,
            generatedTranscripts,
            translationLanguages,
        );
    }

    /**
     * Iterates over all transcripts (manually created + generated).
     */
    [Symbol.iterator](): Iterator<Transcript> {
        let combined = [
            ...Object.values(this._manuallyCreatedTranscripts),
            ...Object.values(this._generatedTranscripts),
        ];
        let pointer = 0;

        return {
            next: (): IteratorResult<Transcript> => {
                if (pointer < combined.length) {
                    return { done: false, value: combined[pointer++] };
                } else {
                    return { done: true, value: null as unknown as Transcript };
                }
            },
        };
    }

    /**
     * Finds the first matching transcript from either the manual or generated sets
     * that matches one of the provided language codes in order.
     */
    public findTranscript(languageCodes: string[]): Transcript {
        return this._findTranscript(languageCodes, [
            this._manuallyCreatedTranscripts,
            this._generatedTranscripts,
        ]);
    }

    /**
     * Finds an automatically generated transcript for a given language code.
     */
    public findGeneratedTranscript(languageCodes: string[]): Transcript {
        return this._findTranscript(languageCodes, [
            this._generatedTranscripts,
        ]);
    }

    /**
     * Finds a manually created transcript for a given language code.
     */
    public findManuallyCreatedTranscript(languageCodes: string[]): Transcript {
        return this._findTranscript(languageCodes, [
            this._manuallyCreatedTranscripts,
        ]);
    }

    private _findTranscript(
        languageCodes: string[],
        transcriptDicts: Array<Record<string, Transcript>>,
    ): Transcript {
        for (const lang of languageCodes) {
            for (const dict of transcriptDicts) {
                if (lang in dict) {
                    return dict[lang];
                }
            }
        }
        throw new NoTranscriptFound(this.videoId, languageCodes, this);
    }

    public toString(): string {
        const man = Object.values(this._manuallyCreatedTranscripts)
            .map((t) => t.toString())
            .join('\n');
        const gen = Object.values(this._generatedTranscripts)
            .map((t) => t.toString())
            .join('\n');
        const transLang = this._translationLanguages
            .map((tl) => `${tl.language_code} ("${tl.language}")`)
            .join('\n');

        return `
For this video (${this.videoId}) transcripts are available in the following languages:

(MANUALLY CREATED)
${man || 'None'}

(GENERATED)
${gen || 'None'}

(TRANSLATION LANGUAGES)
${transLang || 'None'}
    `.trim();
    }
}

export class Transcript {
    public videoId: string;
    public language: string;
    public languageCode: string;
    public isGenerated: boolean;
    public translationLanguages: Array<{
        language: string;
        language_code: string;
    }>;

    private _httpClient: {
        get(
            url: string,
            extraHeaders?: Record<string, string>,
        ): Promise<Response>;
    };
    private _url: string;
    private _translationLanguagesDict: Record<string, string>;

    constructor(
        httpClient: {
            get(
                url: string,
                extraHeaders?: Record<string, string>,
            ): Promise<Response>;
        },
        videoId: string,
        url: string,
        language: string,
        languageCode: string,
        isGenerated: boolean,
        translationLanguages: Array<{
            language: string;
            language_code: string;
        }>,
    ) {
        this._httpClient = httpClient;
        this.videoId = videoId;
        this._url = url;
        this.language = language;
        this.languageCode = languageCode;
        this.isGenerated = isGenerated;
        this.translationLanguages = translationLanguages;
        this._translationLanguagesDict = {};
        for (let tl of translationLanguages) {
            this._translationLanguagesDict[tl.language_code] = tl.language;
        }
    }

    public async fetch(
        preserveFormatting = false,
    ): Promise<Array<TranscriptLine>> {
        const response = await this._httpClient.get(this._url);
        await raiseHttpErrors(response, this.videoId);
        const text = await response.text();
        return new _TranscriptParser(preserveFormatting).parse(text);
    }

    public get isTranslatable(): boolean {
        return this.translationLanguages.length > 0;
    }

    public translate(languageCode: string): Transcript {
        if (!this.isTranslatable) {
            throw new NotTranslatable(this.videoId);
        }
        if (!this._translationLanguagesDict[languageCode]) {
            throw new TranslationLanguageNotAvailable(this.videoId);
        }

        const newUrl = `${this._url}&tlang=${encodeURIComponent(languageCode)}`;
        return new Transcript(
            this._httpClient,
            this.videoId,
            newUrl,
            this._translationLanguagesDict[languageCode],
            languageCode,
            true,
            [],
        );
    }

    public toString(): string {
        const maybeTranslatable = this.isTranslatable ? '[TRANSLATABLE]' : '';
        return `${this.languageCode} ("${this.language}")${maybeTranslatable}`;
    }
}

/**
 * Internal class to parse the raw XML-based transcript from YouTube.
 */
class _TranscriptParser {
    private _htmlRegex: RegExp;
    private _FORMATTING_TAGS = [
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

    constructor(preserveFormatting: boolean) {
        this._htmlRegex = this._getHtmlRegex(preserveFormatting);
    }

    private _getHtmlRegex(preserveFormatting: boolean): RegExp {
        if (preserveFormatting) {
            // Keep tags like <strong>, <b>, <i>, etc.
            const tags = this._FORMATTING_TAGS.join('|');
            const pattern = `</?(?!(${tags})\\b).*?>`;
            return new RegExp(pattern, 'gi');
        } else {
            // Remove all HTML tags
            return new RegExp('<[^>]*>', 'gi');
        }
    }

    public parse(plainData: string): Array<TranscriptLine> {
        // The data is XML: <transcript><text start="..." dur="...">...</text></transcript>
        const parser = new DOMParser();
        const xmlDoc = parser.parseFromString(plainData, 'text/xml');
        const texts = Array.from(xmlDoc.getElementsByTagName('text'));

        return texts
            .filter((elem) => elem.textContent !== null)
            .map((elem) => {
                const startAttr = elem.getAttribute('start') || '0.0';
                const durAttr = elem.getAttribute('dur') || '0.0';
                const rawText = elem.textContent || '';
                const cleanedText = rawText
                    ? rawText.replace(this._htmlRegex, '')
                    : '';

                return {
                    text: unescape(cleanedText),
                    start: parseFloat(startAttr),
                    duration: parseFloat(durAttr),
                };
            });
    }
}
