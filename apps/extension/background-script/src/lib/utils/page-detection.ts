/**
 * Page detection utilities
 */

import { isYouTubeVideoUrl } from './url-helpers.js';

/**
 * Common news and blog domains
 */
const NEWS_DOMAINS = [
	'nytimes.com',
	'washingtonpost.com',
	'theguardian.com',
	'bbc.com',
	'bbc.co.uk',
	'cnn.com',
	'reuters.com',
	'forbes.com',
	'bloomberg.com',
	'medium.com',
	'techcrunch.com',
	'wired.com',
	'vox.com',
	'huffpost.com',
	'theatlantic.com',
	'theverge.com',
	'engadget.com',
	'arstechnica.com'
];

/**
 * Common blog platforms
 */
const BLOG_PLATFORMS = [
	'wordpress.com',
	'blogger.com',
	'medium.com',
	'substack.com',
	'ghost.io',
	'tumblr.com'
];

/**
 * Article indicator paths
 */
const ARTICLE_PATHS = [
	'/article/',
	'/articles/',
	'/story/',
	'/stories/',
	'/news/',
	'/blog/',
	'/opinion/',
	'/post/',
	'/posts/'
];

/**
 * Determines if a URL points to an article page
 * @param {string} url - The URL to check
 * @returns {boolean} True if the URL likely points to an article page
 */
export function isArticlePage(url) {
	try {
		// Skip YouTube videos or empty URLs
		if (!url || isYouTubeVideoUrl(url)) {
			return false;
		}

		const parsedUrl = new URL(url);
		const hostname = parsedUrl.hostname;
		const pathname = parsedUrl.pathname;

		// Check if it's a known news domain
		if (NEWS_DOMAINS.some((domain) => hostname.includes(domain))) {
			return true;
		}

		// Check if it's a known blog platform
		if (BLOG_PLATFORMS.some((domain) => hostname.includes(domain))) {
			return true;
		}

		// Check if the URL path contains article indicators
		if (ARTICLE_PATHS.some((path) => pathname.includes(path))) {
			return true;
		}

		// Check for common article URL patterns
		if (/\d{4}\/\d{1,2}\/\d{1,2}/.test(pathname)) {
			// Matches date patterns like /2023/05/12/
			return true;
		}

		if (/\/\d{4}\/\d{1,2}\//.test(pathname)) {
			// Matches year/month patterns like /2023/05/
			return true;
		}

		// Articles often have slugs with multiple hyphens
		const slugParts = pathname.split('/').filter(Boolean);
		if (slugParts.length > 0) {
			const lastSlugPart = slugParts[slugParts.length - 1];
			if (lastSlugPart.includes('-') && lastSlugPart.split('-').length >= 3) {
				return true;
			}
		}

		return false;
	} catch (e) {
		console.error('Error detecting article page:', e);
		return false;
	}
}
