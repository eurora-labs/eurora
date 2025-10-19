import { defineConfig, devices } from '@playwright/test';

/**
 * Web Extension E2E Testing Configuration
 * Tests the complete browser extension including:
 * - Content scripts
 * - Background scripts (service worker)
 * - Popup window
 *
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
	testDir: './tests',

	/* Run tests in files in parallel */
	fullyParallel: true,

	/* Fail the build on CI if you accidentally left test.only in the source code. */
	forbidOnly: !!process.env['CI'],

	/* Retry on CI only */
	retries: process.env['CI'] ? 2 : 0,

	/* Opt out of parallel tests on CI. */
	workers: process.env['CI'] ? 1 : undefined,

	/* Reporter to use. See https://playwright.dev/docs/test-reporters */
	reporter: [['html', { outputFolder: 'playwright-report' }], ['list']],

	/* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
	use: {
		/* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
		trace: 'on-first-retry',

		/* Screenshot on failure */
		screenshot: 'only-on-failure',

		/* Video on first retry */
		video: 'retain-on-failure',
	},

	/* Configure projects for major browsers */
	projects: [
		{
			name: 'chromium',
			use: {
				...devices['Desktop Chrome'],
				// Extension tests run with chromium channel to support extensions
				channel: 'chromium',
			},
		},

		{
			name: 'firefox',
			use: { ...devices['Desktop Firefox'] },
		},

		{
			name: 'webkit',
			use: { ...devices['Desktop Safari'] },
		},
	],

	/* No web server needed for extension tests - they run against real websites */
});
