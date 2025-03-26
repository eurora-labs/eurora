import { WATCH_URL } from './settings.js';

export class CouldNotRetrieveTranscript extends Error {
    public videoId: string;

    protected static ERROR_MESSAGE =
        '\nCould not retrieve a transcript for the video {video_url}!';

    protected static CAUSE_MESSAGE_INTRO =
        ' This is most likely caused by:\n\n{cause}';
    protected static CAUSE_MESSAGE = '';
    protected static GITHUB_REFERRAL = `
If you are sure that the described cause is not responsible for this error
and that a transcript should be retrievable, please create an issue at
https://github.com/jdepoix/youtube-transcript-api/issues. Please include
the version of youtube_transcript_api you are using and information to replicate the error.
Also make sure there are no open issues already describing your problem!
`;

    constructor(videoId: string, causeMsg?: string) {
        const formatted = CouldNotRetrieveTranscript._buildErrorMessage(
            videoId,
            causeMsg,
        );
        super(formatted);
        this.name = this.constructor.name;
        this.videoId = videoId;
    }

    private static _buildErrorMessage(
        videoId: string,
        causeMsg?: string,
    ): string {
        let message = CouldNotRetrieveTranscript.ERROR_MESSAGE.replace(
            '{video_url}',
            WATCH_URL.replace('{video_id}', videoId),
        );
        if (causeMsg) {
            message +=
                CouldNotRetrieveTranscript.CAUSE_MESSAGE_INTRO.replace(
                    '{cause}',
                    causeMsg,
                ) + CouldNotRetrieveTranscript.GITHUB_REFERRAL;
        }
        return message;
    }
}

export class YouTubeRequestFailed extends CouldNotRetrieveTranscript {
    constructor(videoId: string, httpError: string) {
        super(videoId, `Request to YouTube failed: ${httpError}`);
    }
}

export class VideoUnavailable extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(videoId, 'The video is no longer available');
    }
}

export class InvalidVideoId extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(
            videoId,
            `You provided an invalid video id. Make sure you are using the video id and NOT the url!

Do NOT call: \`YouTubeTranscriptApi.getTranscript("https://www.youtube.com/watch?v=${videoId}")\`
Instead call: \`YouTubeTranscriptApi.getTranscript("${videoId}")\`
`,
        );
    }
}

export class TooManyRequests extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(
            videoId,
            `YouTube is receiving too many requests from this IP and now requires solving a captcha to continue.
One of the following can be done to work around this:
- Manually solve the captcha in a browser and rely on the resulting cookie
- Use a different IP address
- Wait until the ban on your IP has been lifted
`,
        );
    }
}

export class TranscriptsDisabled extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(videoId, 'Subtitles are disabled for this video');
    }
}

export class NoTranscriptAvailable extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(videoId, 'No transcripts are available for this video');
    }
}

export class NotTranslatable extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(videoId, 'The requested language is not translatable');
    }
}

export class TranslationLanguageNotAvailable extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(videoId, 'The requested translation language is not available');
    }
}

export class CookiePathInvalid extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(videoId, 'The provided cookie file was unable to be loaded');
    }
}

export class CookiesInvalid extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(videoId, 'The cookies provided are not valid (may have expired)');
    }
}

export class FailedToCreateConsentCookie extends CouldNotRetrieveTranscript {
    constructor(videoId: string) {
        super(
            videoId,
            'Failed to automatically give consent to saving cookies',
        );
    }
}

export class NoTranscriptFound extends CouldNotRetrieveTranscript {
    constructor(
        videoId: string,
        requestedLanguageCodes: string[],
        transcriptData: unknown,
    ) {
        const msg = `No transcripts were found for any of the requested language codes: ${requestedLanguageCodes.join(
            ', ',
        )}

${JSON.stringify(transcriptData, null, 2)}`;
        super(videoId, msg);
    }
}
