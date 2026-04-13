import { test, expect } from './utils/fixtures.ts';
import { waitForBootstrap, waitForSiteMounted } from './utils/helpers.ts';

test.describe('Content Script Basic Tests', () => {
	test('should load a page with the extension active', async ({ page }) => {
		await page.goto('https://example.com');
		await expect(page).toHaveTitle(/Example Domain/);
	});

	test('bootstrap + mount', async ({ page }) => {
		await page.goto('https://example.com');
		await waitForBootstrap(page);
		await waitForSiteMounted(page, 'default');
	});
});
