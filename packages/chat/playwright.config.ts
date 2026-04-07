import { defineConfig } from '@playwright/test';

export default defineConfig({
	webServer: {
		command: 'pnpm dev --port 4399',
		port: 4399,
		reuseExistingServer: !process.env.CI,
	},
	testDir: 'e2e',
	fullyParallel: true,
	forbidOnly: !!process.env.CI,
	retries: process.env.CI ? 2 : 0,
	workers: process.env.CI ? 1 : undefined,
	reporter: 'html',
	use: {
		baseURL: 'http://localhost:4399',
		trace: 'on-first-retry',
	},
});
