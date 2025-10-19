import { expect, Page } from '@playwright/test';

/**
 * Wait for the content script bootstrap to complete
 *
 * The content script sets eurora-ext-ready="1" on the html element when ready
 */
export async function waitForBootstrap(page: Page) {
	await expect(page.locator('html[eurora-ext-ready="1"]')).toBeVisible();
}

/**
 * Wait for a specific site handler to be mounted
 *
 * @param page - The Playwright page object
 * @param siteId - The site identifier (e.g., 'default', 'youtube.com')
 */
export async function waitForSiteMounted(page: Page, siteId: string) {
	await expect(
		page.locator(`html[eurora-ext-mounted="1"][eurora-ext-site="${siteId}"]`),
	).toBeVisible();
}
