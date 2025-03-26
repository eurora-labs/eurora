/**
 * URL helper functions
 */

/**
 * Check if a URL is a YouTube video page
 */
export function isYouTubeVideoUrl(url: string): boolean {
	try {
		const parsedUrl = new URL(url);
		const isYouTubeHost =
			parsedUrl.hostname === 'www.youtube.com' ||
			parsedUrl.hostname === 'youtube.com' ||
			parsedUrl.hostname === 'm.youtube.com';

		return isYouTubeHost && parsedUrl.pathname === '/watch' && parsedUrl.searchParams.has('v');
	} catch (e) {
		console.error('Invalid URL:', e);
		return false;
	}
}

/**
 * Check if a URL is a PDF page
 */
export function isPdfUrl(url: string): boolean {
	try {
		return url.startsWith('chrome-extension://hmpbdoleeoankjfjcogiohcfojknnkdd/');
	} catch (e) {
		console.error('Invalid URL:', e);
		return false;
	}
}

/**
 * Extracts video ID from a YouTube URL
 * @param {string} url - YouTube URL
 * @returns {string|null} Video ID or null if not found
 */
export function extractYouTubeVideoId(url) {
	try {
		const parsedUrl = new URL(url);
		if (isYouTubeVideoUrl(url)) {
			return parsedUrl.searchParams.get('v');
		}
		return null;
	} catch (e) {
		console.error('Invalid URL:', e);
		return null;
	}
}
