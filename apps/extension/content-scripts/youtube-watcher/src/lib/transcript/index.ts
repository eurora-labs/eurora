// Main entry point for the YouTube Transcript API library
export * from './types.js';
export * from './transcript-api.js';

// Convenience function to get transcript from current YouTube page
export async function getCurrentVideoTranscript(
	languages: string[] = ['en'],
	preserveFormatting: boolean = false,
) {
	const { YouTubeTranscriptApi } = await import('./transcript-api.js');

	// Extract video ID from current page URL
	const videoId = extractVideoIdFromUrl(window.location.href);
	if (!videoId) {
		throw new Error('No YouTube video ID found in current URL');
	}

	const api = new YouTubeTranscriptApi();
	return api.fetch(videoId, languages, preserveFormatting);
}

// Convenience function to list available transcripts for current video
export async function getCurrentVideoTranscriptList() {
	const { YouTubeTranscriptApi } = await import('./transcript-api.js');

	const videoId = extractVideoIdFromUrl(window.location.href);
	if (!videoId) {
		throw new Error('No YouTube video ID found in current URL');
	}

	const api = new YouTubeTranscriptApi();
	return api.list(videoId);
}

// Utility function to extract video ID from YouTube URL
export function extractVideoIdFromUrl(url: string): string | null {
	try {
		const urlObj = new URL(url);

		// Standard watch URL: https://www.youtube.com/watch?v=VIDEO_ID
		if (urlObj.pathname === '/watch') {
			return urlObj.searchParams.get('v');
		}

		// Short URL: https://youtu.be/VIDEO_ID
		if (urlObj.hostname === 'youtu.be') {
			return urlObj.pathname.slice(1);
		}

		// Embed URL: https://www.youtube.com/embed/VIDEO_ID
		if (urlObj.pathname.startsWith('/embed/')) {
			return urlObj.pathname.slice(7);
		}

		return null;
	} catch {
		return null;
	}
}

// Utility function to check if current page is a YouTube video
export function isYouTubeVideoPage(): boolean {
	return (
		window.location.hostname === 'www.youtube.com' &&
		window.location.pathname === '/watch' &&
		window.location.search.includes('v=')
	);
}
