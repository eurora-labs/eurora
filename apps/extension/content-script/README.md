# Content Script Testing

This package contains the content scripts for the browser extension with a comprehensive testing setup.

## Testing Setup

The testing infrastructure uses:

- **Vitest** - Fast unit test framework for unit tests
- **Playwright** - End-to-end testing framework
- **jsdom** - DOM environment for unit testing
- **@testing-library/jest-dom** - Additional DOM matchers

## Running Tests

```bash
# Run tests once
pnpm test

# Run tests in watch mode
pnpm run test:unit

# Run with coverage
pnpm run test:unit -- --coverage
```

### End-to-End Tests

```bash
# Run e2e tests
pnpm run test:e2e

# Run e2e tests in UI mode
pnpm run test:e2e:ui

# Run e2e tests in debug mode
pnpm run test:e2e:debug

# Show test report
pnpm run test:e2e:report
```

## Test Structure

### Unit Tests

Tests are organized alongside the source code:

```
src/
├── __tests__/
│   ├── setup.ts              # Global test setup and mocks
│   └── bootstrap.test.ts     # Bootstrap tests
├── sites/
│   ├── _default/
│   │   └── __tests__/
│   │       └── index.test.ts # Default site handler tests
│   └── youtube.com/
│       └── __tests__/
│           └── index.test.ts # YouTube site handler tests
```

### End-to-End Tests

```
e2e/
├── fixtures/
│   └── extension.ts          # Extension fixture for loading browser extension
├── basic.spec.ts             # Basic browser tests
└── extension.spec.ts         # Extension-specific tests
```

## Test Configuration

### Unit Test Configuration

- **vitest.config.ts** - Main Vitest configuration
- **src/**tests**/setup.ts** - Global test setup including:
    - Browser extension API mocks
    - Canvas API mocks for jsdom
    - DOM environment setup

### E2E Test Configuration

- **playwright.config.ts** - Playwright configuration
- **e2e/fixtures/extension.ts** - Custom fixture for loading the extension in tests

## Writing Tests

### Unit Test Example

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

describe('my feature', () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	it('should work correctly', () => {
		expect(true).toBe(true);
	});
});
```

### Mocking Browser APIs

Browser extension APIs are automatically mocked in the setup file. Access them via:

```typescript
import browser from 'webextension-polyfill';

// Use mocked browser API
browser.runtime.sendMessage({ type: 'TEST' });
```

### E2E Test Example

```typescript
import { test, expect } from '@playwright/test';

test.describe('My Feature', () => {
	test('should work correctly', async ({ page }) => {
		await page.goto('https://example.com');
		await expect(page).toHaveTitle(/Example/);
	});
});
```

### E2E Test with Extension

```typescript
import { test, expect } from './fixtures/extension.js';

test.describe('Extension Feature', () => {
	test('should load extension', async ({ context, extensionId }) => {
		expect(extensionId).toBeTruthy();

		const page = await context.newPage();
		await page.goto('https://example.com');
		// Test extension functionality
		await page.close();
	});
});
```

## Coverage

### Unit Test Coverage

Generate coverage reports with:

```bash
pnpm run test:unit -- --coverage
```

Coverage reports are generated in the `coverage/` directory.
