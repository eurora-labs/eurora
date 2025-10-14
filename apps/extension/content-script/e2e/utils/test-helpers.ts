import { Page, BrowserContext } from '@playwright/test';

/**
 * Helper function to wait for extension to be ready on a page
 */
export async function waitForExtensionReady(page: Page, timeout = 2000): Promise<void> {
	await page.waitForTimeout(timeout);
}

/**
 * Send a message to the extension content script
 */
export async function sendMessageToContentScript(page: Page, message: any): Promise<any> {
	return await page.evaluate(async (msg) => {
		return new Promise((resolve, reject) => {
			// Check if chrome extension APIs are available
			// @ts-ignore
			if (typeof chrome === 'undefined' || !chrome.runtime) {
				reject(
					new Error('Chrome extension APIs not available. Ensure extension is loaded.'),
				);
				return;
			}

			const timeout = setTimeout(() => {
				reject(new Error('Message timeout'));
			}, 5000);

			// @ts-ignore
			chrome.runtime.sendMessage(msg, (response: any) => {
				clearTimeout(timeout);
				// @ts-ignore
				if (chrome.runtime.lastError) {
					// @ts-ignore
					reject(chrome.runtime.lastError);
				} else {
					resolve(response);
				}
			});
		});
	}, message);
}

/**
 * Check if a specific site handler is loaded
 */
export async function isHandlerLoaded(page: Page, handlerName: string): Promise<boolean> {
	return await page.evaluate((name) => {
		// Check for handler-specific markers or functionality
		return (window as any)[`__handler_${name}_loaded__`] === true;
	}, handlerName);
}

/**
 * Get current video time for YouTube tests
 */
export async function getYouTubeVideoTime(page: Page): Promise<number> {
	return await page.evaluate(() => {
		const video = document.querySelector('video.html5-main-video') as HTMLVideoElement;
		return video?.currentTime ?? -1;
	});
}

/**
 * Set YouTube video time
 */
export async function setYouTubeVideoTime(page: Page, time: number): Promise<void> {
	await page.evaluate((t) => {
		const video = document.querySelector('video.html5-main-video') as HTMLVideoElement;
		if (video) {
			video.currentTime = t;
		}
	}, time);
}

/**
 * Check if YouTube video is playing
 */
export async function isYouTubeVideoPlaying(page: Page): Promise<boolean> {
	return await page.evaluate(() => {
		const video = document.querySelector('video.html5-main-video') as HTMLVideoElement;
		return video ? !video.paused : false;
	});
}

/**
 * Get video ID from YouTube URL
 */
export function extractYouTubeVideoId(url: string): string | null {
	const match = url.match(/[?&]v=([^&]+)/);
	return match ? match[1] : null;
}

/**
 * Wait for YouTube video player to be ready
 */
export async function waitForYouTubePlayer(page: Page, timeout = 5000): Promise<boolean> {
	try {
		await page.waitForSelector('video.html5-main-video', { timeout });
		await page.waitForFunction(
			() => {
				const video = document.querySelector('video.html5-main-video') as HTMLVideoElement;
				return video && video.readyState >= 2; // HAVE_CURRENT_DATA
			},
			{ timeout },
		);
		return true;
	} catch {
		return false;
	}
}

/**
 * Collect console messages from a page
 */
export function collectConsoleMessages(page: Page): {
	messages: string[];
	errors: string[];
	warnings: string[];
} {
	const messages: string[] = [];
	const errors: string[] = [];
	const warnings: string[] = [];

	page.on('console', (msg) => {
		const text = msg.text();
		messages.push(text);

		if (msg.type() === 'error') {
			errors.push(text);
		} else if (msg.type() === 'warning') {
			warnings.push(text);
		}
	});

	return { messages, errors, warnings };
}

/**
 * Create a test page with console logging
 */
export async function createTestPage(context: BrowserContext): Promise<{
	page: Page;
	console: { messages: string[]; errors: string[]; warnings: string[] };
}> {
	const page = await context.newPage();
	const consoleData = collectConsoleMessages(page);

	return { page, console: consoleData };
}

/**
 * Verify response structure
 */
export function verifyResponseStructure(
	response: any,
	expectedKind?: string,
): {
	isValid: boolean;
	kind: string | undefined;
	data: any;
} {
	const isValid =
		response !== null &&
		typeof response === 'object' &&
		'kind' in response &&
		'data' in response;

	const kind = response?.kind;
	const data = response?.data;

	if (expectedKind && kind !== expectedKind) {
		return { isValid: false, kind, data };
	}

	return { isValid, kind, data };
}

/**
 * Verify error response
 */
export function verifyErrorResponse(response: any): {
	isError: boolean;
	errorMessage?: string;
} {
	if (!response || typeof response !== 'object') {
		return { isError: false };
	}

	const isError = response.kind === 'Error';
	const errorMessage = isError ? response.data : undefined;

	return { isError, errorMessage };
}

/**
 * Generate test article HTML
 */
export function generateTestArticleHTML(title: string, content: string): string {
	return `
		<!DOCTYPE html>
		<html lang="en">
		<head>
			<meta charset="UTF-8">
			<meta name="viewport" content="width=device-width, initial-scale=1.0">
			<title>${title}</title>
			<meta property="og:title" content="${title}">
			<meta property="og:description" content="${content}">
		</head>
		<body>
			<article>
				<h1>${title}</h1>
				<p>${content}</p>
			</article>
		</body>
		</html>
	`;
}

/**
 * Test helper for message type routing
 */
export const MessageTypes = {
	NEW: 'NEW',
	PLAY: 'PLAY',
	GENERATE_ASSETS: 'GENERATE_ASSETS',
	GENERATE_SNAPSHOT: 'GENERATE_SNAPSHOT',
} as const;

/**
 * Test URLs for different scenarios
 */
export const TestURLs = {
	example: 'https://example.com',
	youtube: {
		video: 'https://www.youtube.com/watch?v=dQw4w9WgXcQ',
		home: 'https://www.youtube.com',
		channel: 'https://www.youtube.com/@channel',
	},
	article: 'https://example.com/article',
} as const;

/**
 * Wait for specific message to be logged
 */
export async function waitForConsoleMessage(
	page: Page,
	matcher: string | RegExp,
	timeout = 5000,
): Promise<boolean> {
	return new Promise((resolve) => {
		const timeoutId = setTimeout(() => resolve(false), timeout);

		const handler = (msg: any) => {
			const text = msg.text();
			const matches =
				typeof matcher === 'string' ? text.includes(matcher) : matcher.test(text);

			if (matches) {
				clearTimeout(timeoutId);
				page.off('console', handler);
				resolve(true);
			}
		};

		page.on('console', handler);
	});
}

/**
 * Retry an operation with exponential backoff
 */
export async function retryOperation<T>(
	operation: () => Promise<T>,
	maxRetries = 3,
	initialDelay = 1000,
): Promise<T> {
	let lastError: Error | undefined;

	for (let i = 0; i < maxRetries; i++) {
		try {
			return await operation();
		} catch (error) {
			lastError = error as Error;
			if (i < maxRetries - 1) {
				await new Promise((resolve) => setTimeout(resolve, initialDelay * Math.pow(2, i)));
			}
		}
	}

	throw lastError || new Error('Operation failed after retries');
}

/**
 * Mock extension API responses
 */
export async function mockExtensionAPI(page: Page, mocks: Record<string, any>): Promise<void> {
	await page.addInitScript((mockData) => {
		// @ts-ignore
		window.__extensionMocks__ = mockData;
	}, mocks);
}

/**
 * Clean up test page
 */
export async function cleanupTestPage(page: Page): Promise<void> {
	try {
		await page.close();
	} catch (error) {
		console.error('Error closing page:', error);
	}
}

/**
 * Assert YouTube asset structure
 */
export function assertYouTubeAssetStructure(data: any): boolean {
	return (
		typeof data === 'object' &&
		typeof data.url === 'string' &&
		typeof data.title === 'string' &&
		typeof data.current_time === 'number' &&
		data.url.includes('youtube.com')
	);
}

/**
 * Assert YouTube snapshot structure
 */
export function assertYouTubeSnapshotStructure(data: any): boolean {
	return (
		typeof data === 'object' &&
		typeof data.current_time === 'number' &&
		typeof data.video_frame_base64 === 'string' &&
		typeof data.video_frame_width === 'number' &&
		typeof data.video_frame_height === 'number' &&
		data.video_frame_width > 0 &&
		data.video_frame_height > 0 &&
		data.video_frame_base64.length > 0
	);
}

/**
 * Assert article asset structure
 */
export function assertArticleAssetStructure(data: any): boolean {
	return (
		typeof data === 'object' && typeof data.url === 'string' && typeof data.title === 'string'
	);
}
