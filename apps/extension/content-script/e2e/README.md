# Content Script E2E Testing Guide

This directory contains comprehensive end-to-end (E2E) tests for the content script package using Playwright.

## Overview

The E2E test suite validates the entire content script system, including:

- Bootstrap mechanism and dynamic script loading
- Registry system and domain matching
- Site-specific handlers (Default/Article and YouTube)
- Message routing between background and content scripts
- Cross-domain functionality
- Error handling and edge cases

## Test Structure

```
e2e/
├── README.md                        # This file
├── basic.spec.ts                    # Basic infrastructure tests
├── bootstrap.e2e.spec.ts           # Bootstrap mechanism tests
├── registry.e2e.spec.ts            # Registry and domain matching tests
├── site-handlers.e2e.spec.ts       # Site handler functionality tests
├── message-routing.e2e.spec.ts     # Message routing tests
├── extension.spec.ts               # General extension tests
├── fixtures/
│   └── extension.ts                # Extension fixture for loading extension
└── utils/
    └── test-helpers.ts             # Reusable test utility functions
```

## Prerequisites

Before running E2E tests, you must build the extension:

```bash
# Build the content script package
pnpm run build
```

This generates the necessary files in `extensions/chromium/scripts/content/`:

- `bootstrap.js` - The main bootstrap script
- `sites/*/index.js` - Site-specific handler scripts
- `registry.json` - Domain-to-handler mapping

## Running Tests

### Run All E2E Tests

```bash
pnpm run test:e2e
```

### Run in UI Mode (Interactive)

```bash
pnpm run test:e2e:ui
```

This opens Playwright's UI where you can:

- See tests as they run
- Time travel through test steps
- Inspect the DOM at each step
- Debug failing tests

### Run in Debug Mode

```bash
pnpm run test:e2e:debug
```

### Run Specific Test File

```bash
pnpm exec playwright test e2e/bootstrap.e2e.spec.ts
```

### Run Specific Test

```bash
pnpm exec playwright test -g "should respond to SITE_LOAD messages"
```

### View Test Report

```bash
pnpm run test:e2e:report
```

## Test Categories

### 1. Bootstrap Tests (`bootstrap.e2e.spec.ts`)

Tests the core bootstrap mechanism that loads site handlers dynamically.

**Key Tests:**

- Bootstrap script injection
- SITE_LOAD message handling
- Single-load enforcement
- Fallback to default handler
- `canHandle` function support

**Example:**

```typescript
test('should respond to SITE_LOAD messages', async ({ context, extensionId }) => {
	const page = await context.newPage();
	await page.goto('https://example.com');

	const response = await sendMessageToContentScript(page, {
		type: 'SITE_LOAD',
		chunk: 'sites/_default/index.js',
		defaultChunk: 'sites/_default/index.js',
	});

	expect(response.loaded).toBe(true);
});
```

### 2. Registry Tests (`registry.e2e.spec.ts`)

Validates the registry system that maps domains to handlers.

**Key Tests:**

- Registry file generation
- Domain pattern matching
- Wildcard pattern support
- Subdomain matching
- Default handler fallback

**Example:**

```typescript
test('should match youtube.com domain correctly', async ({ context }) => {
	const page = await context.newPage();
	await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');

	// Verify YouTube handler is loaded
	const hasYoutubeHandler = await page.evaluate(() => {
		return document.querySelector('video.html5-main-video') !== null;
	});

	expect(hasYoutubeHandler).toBe(true);
});
```

### 3. Site Handler Tests (`site-handlers.e2e.spec.ts`)

Tests individual site handlers and their functionality.

**Default Handler Tests:**

- Article asset generation
- Snapshot generation
- Metadata extraction
- Error handling

**YouTube Handler Tests:**

- Video detection
- Transcript fetching
- Video frame capture
- Timestamp tracking
- Video playback control

**Example:**

```typescript
test('should handle GENERATE_ASSETS for YouTube video', async ({ context }) => {
	const page = await context.newPage();
	await page.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');
	await waitForYouTubePlayer(page);

	const response = await sendMessageToContentScript(page, {
		type: 'GENERATE_ASSETS',
	});

	expect(response.kind).toBe('NativeYoutubeAsset');
	expect(response.data.url).toContain('youtube.com');
});
```

### 4. Message Routing Tests (`message-routing.e2e.spec.ts`)

Validates message passing and routing between components.

**Key Tests:**

- Domain-based routing
- Message type routing
- Invalid message handling
- Concurrent messages
- Cross-tab isolation

**Example:**

```typescript
test('should route messages to correct handler based on domain', async ({ context }) => {
	const ytPage = await context.newPage();
	await ytPage.goto('https://www.youtube.com/watch?v=dQw4w9WgXcQ');

	const ytResponse = await sendMessageToContentScript(ytPage, {
		type: 'GENERATE_ASSETS',
	});

	expect(ytResponse.kind).toBe('NativeYoutubeAsset');
});
```

## Test Utilities

The `utils/test-helpers.ts` file provides reusable utilities:

### Message Handling

```typescript
import { sendMessageToContentScript } from './utils/test-helpers';

const response = await sendMessageToContentScript(page, {
	type: 'GENERATE_ASSETS',
});
```

### YouTube Helpers

```typescript
import {
	waitForYouTubePlayer,
	getYouTubeVideoTime,
	setYouTubeVideoTime,
} from './utils/test-helpers';

await waitForYouTubePlayer(page);
const time = await getYouTubeVideoTime(page);
await setYouTubeVideoTime(page, 30);
```

### Response Validation

```typescript
import { verifyResponseStructure, verifyErrorResponse } from './utils/test-helpers';

const { isValid, kind, data } = verifyResponseStructure(response, 'NativeYoutubeAsset');
const { isError, errorMessage } = verifyErrorResponse(response);
```

### Console Monitoring

```typescript
import { collectConsoleMessages, waitForConsoleMessage } from './utils/test-helpers';

const { messages, errors, warnings } = collectConsoleMessages(page);
const found = await waitForConsoleMessage(page, 'Article Watcher');
```

## Extension Fixture

The extension fixture (`fixtures/extension.ts`) provides a Playwright browser context with the extension pre-loaded:

```typescript
import { test, expect } from './fixtures/extension.js';

test('my test', async ({ context, extensionId }) => {
	const page = await context.newPage();
	// Extension is already loaded
	await page.goto('https://example.com');
	// Test extension functionality
});
```

## Test Configuration

Tests are configured in `playwright.config.ts`:

- **Browsers**: Chromium, Firefox, WebKit
- **Parallelization**: Enabled (disabled on CI)
- **Retries**: 2 on CI, 0 locally
- **Trace**: On first retry
- **Screenshots**: On failure

## Debugging Tests

### Using Browser DevTools

```bash
pnpm run test:e2e:debug
```

Then use Chrome DevTools to:

- Set breakpoints in test code
- Inspect page state
- View console logs
- Step through execution

### Using Playwright Inspector

The debug mode automatically opens Playwright Inspector where you can:

- Step through test actions
- See selector highlights
- View action logs
- Time travel through test

### Adding Debug Points

Add `await page.pause()` in your test:

```typescript
test('debug test', async ({ page }) => {
	await page.goto('https://example.com');
	await page.pause(); // Execution stops here
	// Continue testing...
});
```

## Common Issues

### Extension Not Loaded

**Problem**: Tests fail because extension isn't loaded.

**Solution**: Ensure you've built the extension first:

```bash
pnpm run build
```

### Tests Marked as .skip

**Problem**: Many tests are skipped by default.

**Reason**: Tests require the extension to be built and properly configured.

**Solution**:

1. Build the extension
2. Remove `.skip` from tests you want to run
3. Ensure extension path is correct in fixture

### Timeout Errors

**Problem**: Tests timeout waiting for elements or actions.

**Solution**:

- Increase timeout values for slow operations
- Add proper wait conditions
- Use `waitForYouTubePlayer()` for video tests

### Console Errors

**Problem**: Extension errors appear in console.

**Solution**:

- Check bootstrap.js is loading correctly
- Verify registry.json exists
- Ensure site handler files are present

## Best Practices

1. **Always build before testing**: Run `pnpm run build` before E2E tests
2. **Use test helpers**: Import utilities from `utils/test-helpers.ts`
3. **Wait for readiness**: Use `waitForExtensionReady()` or `waitForYouTubePlayer()`
4. **Clean up**: Always close pages after tests
5. **Meaningful assertions**: Test behavior, not implementation details
6. **Isolate tests**: Each test should be independent
7. **Use proper selectors**: Prefer data-testid or stable selectors

## CI/CD Integration

Tests are configured to run in CI with:

- Serial execution (no parallelization)
- Retry on failure (2 retries)
- Trace collection on failure
- HTML report generation

## Writing New Tests

### Template for Site Handler Test

```typescript
import { test, expect } from './fixtures/extension.js';
import { sendMessageToContentScript, waitForExtensionReady } from './utils/test-helpers';

test.describe('My New Handler Tests', () => {
	test.skip('should handle custom message', async ({ context, extensionId }) => {
		const page = await context.newPage();
		await page.goto('https://example.com');
		await waitForExtensionReady(page);

		const response = await sendMessageToContentScript(page, {
			type: 'MY_MESSAGE_TYPE',
		});

		expect(response).toBeTruthy();
		expect(response.kind).toBe('Success');

		await page.close();
	});
});
```

### Template for Domain Matching Test

```typescript
test.skip('should match new domain pattern', async ({ context }) => {
	const page = await context.newPage();
	await page.goto('https://newsite.com');
	await waitForExtensionReady(page);

	// Verify specific handler is loaded
	const handlerLoaded = await page.evaluate(() => {
		// Check for handler-specific marker
		return true;
	});

	expect(handlerLoaded).toBe(true);
	await page.close();
});
```

## Additional Resources

- [Playwright Documentation](https://playwright.dev/docs/intro)
- [Playwright Best Practices](https://playwright.dev/docs/best-practices)
- [Browser Extension Testing](https://playwright.dev/docs/chrome-extensions)
- [Project README](../README.md)

## Support

For questions or issues with E2E tests:

1. Check this documentation
2. Review test utilities in `utils/test-helpers.ts`
3. Examine existing test examples
4. Check Playwright documentation
