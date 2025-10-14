import { test, expect } from './fixtures/extension.js';

test.describe('Default Site Handler E2E Tests', () => {
	test('should handle GENERATE_ASSETS message on article pages', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		// Simulate GENERATE_ASSETS message
		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'GENERATE_ASSETS',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response).toBeTruthy();
		expect((response as any).kind).toBeDefined();
		await page.close();
	});

	test.skip('should handle GENERATE_SNAPSHOT message', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'GENERATE_SNAPSHOT',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response).toBeTruthy();
		await page.close();
	});

	test.skip('should extract article metadata', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'GENERATE_ASSETS',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		if (response && typeof response === 'object' && (response as any).data) {
			const data = (response as any).data;
			expect(data.url).toBeTruthy();
			expect(data.title).toBeTruthy();
		}

		await page.close();
	});

	test.skip('should return error for invalid message type', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'INVALID_TYPE',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response).toBeTruthy();
		expect((response as any).kind).toBe('Error');
		expect((response as any).data).toContain('Invalid message type');
		await page.close();
	});
});

test.describe('YouTube Site Handler E2E Tests', () => {
	test.skip('should detect YouTube video page', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		const isVideoPage = await page.evaluate(() => {
			return (
				window.location.href.includes('/watch?v=') &&
				document.querySelector('video.html5-main-video') !== null
			);
		});

		expect(isVideoPage).toBe(true);
		await page.close();
	});

	test.skip('should extract video ID from URL', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(2000);

		const videoId = await page.evaluate(() => {
			if (window.location.search?.includes('v=')) {
				return window.location.search.split('v=')[1].split('&')[0];
			}
			return null;
		});

		expect(videoId).toBe('dQw4w9WgXcQ');
		await page.close();
	});

	test.skip('should handle NEW message for YouTube videos', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'NEW',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		// NEW message might not return data, but should process successfully
		// Check console for transcript fetch attempts
		await page.close();
	});

	test.skip('should handle GENERATE_ASSETS for YouTube video', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'GENERATE_ASSETS',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response).toBeTruthy();
		if ((response as any).kind === 'NativeYoutubeAsset') {
			const data = (response as any).data;
			expect(data.url).toBeTruthy();
			expect(data.title).toBeTruthy();
			expect(data.url).toContain('youtube.com');
		}

		await page.close();
	});

	test.skip('should handle GENERATE_SNAPSHOT for YouTube video', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'GENERATE_SNAPSHOT',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response).toBeTruthy();
		if ((response as any).kind === 'NativeYoutubeSnapshot') {
			const data = (response as any).data;
			expect(data.current_time).toBeDefined();
			expect(data.video_frame_base64).toBeTruthy();
			expect(data.video_frame_width).toBeGreaterThan(0);
			expect(data.video_frame_height).toBeGreaterThan(0);
		}

		await page.close();
	});

	test.skip('should handle PLAY message to seek video', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		// Get initial time
		const initialTime = await page.evaluate(() => {
			const video = document.querySelector('video.html5-main-video') as HTMLVideoElement;
			return video?.currentTime || 0;
		});

		// Send PLAY message to seek to 10 seconds
		await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'PLAY',
						value: 10,
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		await page.waitForTimeout(500);

		// Check if video time changed
		const newTime = await page.evaluate(() => {
			const video = document.querySelector('video.html5-main-video') as HTMLVideoElement;
			return video?.currentTime || 0;
		});

		expect(Math.abs(newTime - 10)).toBeLessThan(2); // Allow 2 second tolerance
		await page.close();
	});

	test.skip('should fall back to article handler on non-video YouTube pages', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		// Navigate to YouTube home page (not a video page)
		await page.goto('https://www.youtube.com');
		await page.waitForTimeout(2000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'GENERATE_ASSETS',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response).toBeTruthy();
		// Should return article asset instead of YouTube asset
		if ((response as any).kind === 'NativeArticleAsset') {
			expect((response as any).data.url).toContain('youtube.com');
		}

		await page.close();
	});

	test.skip('should get current video time correctly', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		const currentTime = await page.evaluate(() => {
			const video = document.querySelector('video.html5-main-video') as HTMLVideoElement;
			return video?.currentTime || -1;
		});

		expect(currentTime).toBeGreaterThanOrEqual(0);
		await page.close();
	});

	test.skip('should capture video frame as base64', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'GENERATE_SNAPSHOT',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		if ((response as any).kind === 'NativeYoutubeSnapshot') {
			const data = (response as any).data;
			// Verify base64 string format
			expect(data.video_frame_base64).toMatch(/^[A-Za-z0-9+/=]+$/);
			expect(data.video_frame_base64.length).toBeGreaterThan(100);
		}

		await page.close();
	});
});
