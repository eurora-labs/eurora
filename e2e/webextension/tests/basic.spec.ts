import { test, expect } from './utils/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted } from './utils/helpers.ts';

test.describe('Content Script Basic Tests', () => {
	test('should load a basic webpage', async ({ page }) => {
		await page.goto('https://example.com');
		await expect(page).toHaveTitle(/Example Domain/);
	});

	test('should have document object available', async ({ page }) => {
		await page.goto('https://example.com');

		const hasDocument = await page.evaluate(() => {
			return typeof document !== 'undefined';
		});

		expect(hasDocument).toBe(true);
	});

	test('should be able to query DOM elements', async ({ page }) => {
		await page.goto('https://example.com');

		const heading = page.locator('h1').first();
		expect(heading).toBeVisible();
	});

	test('should have window object available', async ({ page }) => {
		await page.goto('https://example.com');

		const hasWindow = await page.evaluate(() => {
			return typeof window !== 'undefined' && typeof window.location !== 'undefined';
		});

		expect(hasWindow).toBe(true);
	});

	test('should be able to get page URL', async ({ page }) => {
		await page.goto('https://example.com');

		const url = await page.evaluate(() => window.location.href);
		expect(url).toContain('example.com');
	});

	test('bootstrap + mount', async ({ page }) => {
		await page.goto('https://example.com');
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'default');
	});
});
