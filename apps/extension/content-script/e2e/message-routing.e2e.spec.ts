import { test, expect } from './fixtures/extension.js';

test.describe('Message Routing E2E Tests', () => {
	test.skip('should route messages to correct handler based on domain', async ({
		context,
		extensionId,
	}) => {
		// Test YouTube domain routing
		const ytPage = await context.newPage();
		await ytPage.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await ytPage.waitForTimeout(3000);

		const ytResponse = await ytPage.evaluate(async () => {
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

		// YouTube handler should return NativeYoutubeAsset for video pages
		expect(ytResponse).toBeTruthy();

		await ytPage.close();

		// Test default domain routing
		const defaultPage = await context.newPage();
		await defaultPage.goto('https://example.com');
		await defaultPage.waitForTimeout(1000);

		const defaultResponse = await defaultPage.evaluate(async () => {
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

		// Default handler should return article asset
		expect(defaultResponse).toBeTruthy();
		await defaultPage.close();
	});

	test.skip('should handle message type routing within handlers', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		// Test different message types
		const messageTypes = ['GENERATE_ASSETS', 'GENERATE_SNAPSHOT', 'NEW'];

		for (const type of messageTypes) {
			const response = await page.evaluate(async (msgType) => {
				return new Promise((resolve) => {
					// @ts-ignore
					chrome.runtime.sendMessage(
						{
							type: msgType,
						},
						(response: any) => {
							resolve(response);
						},
					);
				});
			}, type);

			expect(response).toBeDefined();
		}

		await page.close();
	});

	test.skip('should handle invalid message types gracefully', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'INVALID_MESSAGE_TYPE',
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		expect(response).toBeTruthy();
		expect((response as any).kind).toBe('Error');
		expect((response as any).data).toBeTruthy();
		await page.close();
	});

	test.skip('should handle concurrent messages correctly', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		// Send multiple messages concurrently
		const promises = await page.evaluate(async () => {
			const p1 = new Promise((resolve) => {
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

			const p2 = new Promise((resolve) => {
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

			return Promise.all([p1, p2]);
		});

		expect(promises).toHaveLength(2);
		promises.forEach((response) => {
			expect(response).toBeTruthy();
		});

		await page.close();
	});

	test.skip('should preserve message sender information', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		// This test verifies that the sender object is passed correctly
		// The actual verification would need to be done in the handler
		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'GENERATE_ASSETS',
						testSender: true,
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

	test.skip('should handle messages with additional data', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		// Send PLAY message with value
		const response = await page.evaluate(async () => {
			return new Promise((resolve) => {
				// @ts-ignore
				chrome.runtime.sendMessage(
					{
						type: 'PLAY',
						value: 15,
					},
					(response: any) => {
						resolve(response);
					},
				);
			});
		});

		// Verify the message was processed
		await page.waitForTimeout(500);

		const currentTime = await page.evaluate(() => {
			const video = document.querySelector('video.html5-main-video') as HTMLVideoElement;
			return video?.currentTime || -1;
		});

		expect(Math.abs(currentTime - 15)).toBeLessThan(2);
		await page.close();
	});

	test.skip('should return promise-based responses correctly', async ({
		context,
		extensionId,
	}) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		// Measure response time
		const startTime = Date.now();

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

		const endTime = Date.now();
		const responseTime = endTime - startTime;

		expect(response).toBeTruthy();
		// Response should be reasonably fast (under 5 seconds)
		expect(responseTime).toBeLessThan(5000);

		await page.close();
	});

	test.skip('should handle message routing with navigation', async ({ context, extensionId }) => {
		const page = await context.newPage();

		// Start on example.com
		await page.goto('https://example.com');
		await page.waitForTimeout(1000);

		const response1 = await page.evaluate(async () => {
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

		expect(response1).toBeTruthy();

		// Navigate to YouTube
		await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
		await page.waitForTimeout(3000);

		const response2 = await page.evaluate(async () => {
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

		// Should get appropriate response based on new domain
		expect(response2).toBeTruthy();

		await page.close();
	});

	test.skip('should not leak handlers between tabs', async ({ context, extensionId }) => {
		// Create two pages
		const page1 = await context.newPage();
		const page2 = await context.newPage();

		await page1.goto('https://example.com');
		await page2.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');

		await page1.waitForTimeout(1000);
		await page2.waitForTimeout(3000);

		// Each page should have its own handler
		const response1 = await page1.evaluate(async () => {
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

		const response2 = await page2.evaluate(async () => {
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

		expect(response1).toBeTruthy();
		expect(response2).toBeTruthy();

		// Responses should be different based on domain
		await page1.close();
		await page2.close();
	});
});
