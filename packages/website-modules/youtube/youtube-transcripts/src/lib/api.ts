/* eslint-disable */

import { TranscriptListFetcher } from './transcripts.js';
// import {
//     CookiePathInvalid,
//     CookiesInvalid,
//     CouldNotRetrieveTranscript,
// } from './errors.js';

/**
 * A front-end replacement for Python's requests.Session-like handling.
 * Here we simply store proxy URLs (if you decide to implement them).
 * Also, cookies are handled automatically by the browser if same-site
 * or cross-site rules allow it.
 */
export interface FetchOptions {
	proxies?: {
		http?: string;
		https?: string;
	};
	/**
	 * In Python version, `cookies` was a path to a file.
	 * In the browser, we typically don't do that.
	 * This field can be a placeholder if needed, or
	 * you can handle manual cookie strings.
	 */
	cookies?: string | null;
}

export class YouTubeTranscriptApi {
	/**
	 * Retrieve the list of transcripts for a given video.
	 * @param videoId The Youtube video ID
	 * @param proxies The HTTP/HTTPS proxy URLs (not always meaningful in browsers)
	 * @param cookies  Path or string for cookies (in a browser, typically not used)
	 */
	public static async listTranscripts(
		videoId: string,
		proxies?: FetchOptions['proxies'],
		cookies?: FetchOptions['cookies'],
	): Promise<any /* Actually returns a TranscriptList object */> {
		// The original Python code uses a requests.Session and loads cookies from a file.
		// In the browser, cookies are automatically managed or must be attached in headers manually.
		// For illustration, we'll simply pass relevant headers or do nothing special with `cookies`.

		// If you have to handle a custom cookie string, you'd do something like:
		//   const headers = { 'Cookie': cookies ?? '' };
		//   but typically browsers manage cookies by default.

		const httpClient = {
			get: async (url: string, extraHeaders?: Record<string, string>) => {
				// Basic fetch usage in TypeScript
				const resp = await fetch(url, {
					method: 'GET',
					headers: {
						'Accept-Language': 'en-US',
						// If you needed to pass cookies manually, you might do:
						// Cookie: cookies ?? "",
						...(extraHeaders ?? {}),
					},
					// No direct 'proxy' usage in the browser unless you set up a custom proxy server
				});
				return resp;
			},
		};

		// We do not handle cookie loading from a file in the browser environment
		// because it's not feasible. If needed, you can parse cookies from a user-supplied string.

		// We mimic the original logic but skip the cookie jar approach:
		return await new TranscriptListFetcher(httpClient).fetch(videoId);
	}

	/**
	 * Retrieves transcripts for multiple video IDs.
	 */
	public static async getTranscripts(
		videoIds: string[],
		languages: string[] = ['en'],
		continueAfterError = false,
		proxies?: FetchOptions['proxies'],
		cookies?: FetchOptions['cookies'],
		preserveFormatting = false,
	): Promise<[Record<string, any[]>, string[]]> {
		// data: videoId -> transcript array
		const data: Record<string, any[]> = {};
		// unretrievable videos
		const unretrievableVideos: string[] = [];

		for (const videoId of videoIds) {
			try {
				const transcript = await this.getTranscript(
					videoId,
					languages,
					proxies,
					cookies,
					preserveFormatting,
				);
				data[videoId] = transcript;
			} catch (error) {
				if (!continueAfterError) {
					throw error;
				}
				unretrievableVideos.push(videoId);
			}
		}

		return [data, unretrievableVideos];
	}

	/**
	 * Retrieves the transcript for a single video, attempting the provided languages in sequence.
	 */
	public static async getTranscript(
		videoId: string,
		languages: string[] = ['en'],
		proxies?: FetchOptions['proxies'],
		cookies?: FetchOptions['cookies'],
		preserveFormatting = false,
	): Promise<any[]> {
		// This is a convenience method that calls `listTranscripts(...)`
		// and tries to find and fetch the correct one.
		if (typeof videoId !== 'string') {
			throw new Error('`videoId` must be a string');
		}

		const transcriptList = await this.listTranscripts(videoId, proxies, cookies);
		const transcript = transcriptList.findTranscript(languages);
		return transcript.fetch(preserveFormatting);
	}
}
