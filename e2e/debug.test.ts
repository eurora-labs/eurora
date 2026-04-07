import { test } from '@playwright/test';
test('check data-slot', async ({ page }) => {
	page.on('console', (msg) => console.log('BROWSER:', msg.text()));
	await page.goto('/message-list');
	await page.waitForSelector('[data-testid="debug-panel"]');
	await page.evaluate(async () => {
		const t = (window as any).__test;
		return t.setupThread('thread-1', [
			t.makeMessageNode('h1', 'human', 'Hello'),
			t.makeMessageNode('a1', 'ai', 'Hi!', { parentId: 'h1' }),
		]);
	});
	await page.waitForSelector('[data-slot="message"]');
	const msg = page.locator('[data-message-id="a1"]');
	await msg.hover();
	await page.waitForTimeout(200);
	const html = await msg.innerHTML();
	console.log('AI MESSAGE HTML:', html);
	const actionSlots = await msg.locator('[data-slot="message-action"]').count();
	const buttons = await msg.locator('button').count();
	console.log('data-slot=message-action count:', actionSlots);
	console.log('button count:', buttons);
});
