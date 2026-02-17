export * from './types.js';
export * from './transcript-api.js';

export async function getCurrentVideoTranscript(
	languages: string[] = ['en'],
	preserveFormatting: boolean = false,
) {
	const { YouTubeTranscriptApi } = await import('./transcript-api.js');

	const videoId = extractVideoIdFromUrl(window.location.href);
	if (!videoId) {
		throw new Error('No YouTube video ID found in current URL');
	}

	const api = new YouTubeTranscriptApi();
	return await api.fetch(videoId, languages, preserveFormatting);
}

export async function getCurrentVideoTranscriptList() {
	const { YouTubeTranscriptApi } = await import('./transcript-api.js');

	const videoId = extractVideoIdFromUrl(window.location.href);
	if (!videoId) {
		throw new Error('No YouTube video ID found in current URL');
	}

	const api = new YouTubeTranscriptApi();
	return await api.list(videoId);
}

export function extractVideoIdFromUrl(url: string): string | null {
	try {
		const urlObj = new URL(url);

		if (urlObj.pathname === '/watch') {
			return urlObj.searchParams.get('v');
		}

		if (urlObj.hostname === 'youtu.be') {
			return urlObj.pathname.slice(1);
		}

		if (urlObj.pathname.startsWith('/embed/')) {
			return urlObj.pathname.slice(7);
		}

		return null;
	} catch {
		return null;
	}
}

export function isYouTubeVideoPage(): boolean {
	return (
		window.location.hostname === 'www.youtube.com' &&
		window.location.pathname === '/watch' &&
		window.location.search.includes('v=')
	);
}
