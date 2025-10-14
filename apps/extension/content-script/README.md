# Content Script Testing

This package contains the content scripts for the browser extension with a comprehensive testing setup.

## Testing Setup

The testing infrastructure uses:

- **Vitest** - Fast unit test framework
- **jsdom** - DOM environment for testing
- **@testing-library/jest-dom** - Additional DOM matchers

## Running Tests

```bash
# Run tests once
npm test

# Run tests in watch mode
npm run test:unit

# Run with coverage
npm run test:unit -- --coverage
```

## Test Structure

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

## Test Configuration

- **vitest.config.ts** - Main Vitest configuration
- **src/**tests**/setup.ts** - Global test setup including:
    - Browser extension API mocks
    - Canvas API mocks for jsdom
    - DOM environment setup

## Writing Tests

### Basic Test Example

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

## Coverage

Generate coverage reports with:

```bash
npm run test:unit -- --coverage
```

Coverage reports are generated in the `coverage/` directory.
