import { expect, Page } from '@playwright/test';

export async function waitForBootstrap(page: Page) {
	await expect(page.locator('html[eurora-ext-ready="1"]')).toBeVisible();
}

export async function waitForSiteMounted(page: Page, siteId: string) {
	await expect(
		page.locator(`html[eurora-ext-mounted="1"][eurora-ext-site="${siteId}"]`),
	).toBeVisible();
}
