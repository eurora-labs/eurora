import { defineConfig } from '@playwright/test';

const ci = !!process.env.CI;

export default defineConfig({
	webServer: {
		command: ci ? 'pnpm preview --port 4399' : 'pnpm dev --port 4399',
		port: 4399,
		reuseExistingServer: !ci,
	},
	testDir: 'e2e',
	fullyParallel: true,
	forbidOnly: ci,
	retries: ci ? 2 : 0,
	workers: ci ? 1 : undefined,
	reporter: 'html',
	use: {
		baseURL: 'http://localhost:4399',
		trace: 'on-first-retry',
	},
});
