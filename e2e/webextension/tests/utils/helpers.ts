import { expect, type Page } from '@playwright/test';
import type { WatcherResponse } from './types.ts';
import type { Worker } from '@playwright/test';

export async function waitForBootstrap(page: Page) {
	await expect(page.locator('html')).toHaveAttribute('eurora-ext-ready', '1');
}

export async function waitForSiteMounted(page: Page, siteId: string) {
	await expect(page.locator('html')).toHaveAttribute('eurora-ext-mounted', '1');
	await expect(page.locator('html')).toHaveAttribute('eurora-ext-site', siteId);
}

export async function sendToActiveTab(
	sw: Worker,
	message: { type: string },
): Promise<WatcherResponse> {
	return await sw.evaluate(async (msg) => {
		const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
		const response = await chrome.tabs.sendMessage(tab.id!, msg);
		return response;
	}, message);
}
