# Article Watcher Content Script

A content script for the Eurora browser extension that watches and processes article pages.

## Overview

The Article Watcher monitors article pages and provides functionality to:

- Detect new articles
- Generate article assets (metadata, content extraction)
- Create article snapshots

## Development

### Prerequisites

- Node.js (version specified in `.nvmrc`)
- pnpm package manager

### Installation

```bash
pnpm install
```

### Available Scripts

- `pnpm dev` - Start development mode with Vite
- `pnpm build` - Build the content script for production
- `pnpm test` - Run unit tests once
- `pnpm test:unit` - Run unit tests in watch mode
- `pnpm lint` - Check code style and linting
- `pnpm format` - Format code with Prettier

### Testing

The project uses Vitest for unit testing with the following setup:

- **Test Framework**: Vitest
- **Environment**: jsdom (for DOM testing)
- **Coverage**: V8 provider
- **Setup File**: `vitest-setup.ts` (mocks Chrome APIs)

Run tests:

```bash
pnpm test
```

Run tests in watch mode:

```bash
pnpm test:unit
```

### Project Structure

```
article-watcher/
├── src/
│   ├── lib/
│   │   ├── article-watcher.ts      # Main watcher implementation
│   │   ├── article-watcher.test.ts # Unit tests for watcher
│   │   └── types.ts                # TypeScript type definitions
│   ├── index.ts                    # Entry point
│   └── index.test.ts               # Integration tests
├── dist/                           # Build output
├── eslint.config.js                # ESLint configuration
├── package.json                    # Package configuration
├── tsconfig.json                   # TypeScript configuration
├── vite.config.ts                  # Vite build configuration
├── vitest.config.ts                # Vitest test configuration
└── vitest-setup.ts                 # Test setup and mocks
```

### Build Output

The build process:

1. Compiles TypeScript to JavaScript
2. Generates type declarations
3. Copies output to extension directories:
    - `extensions/chromium/content-scripts/article-watcher/`
    - `extensions/firefox/content-scripts/article-watcher/`

## Architecture

### ArticleWatcher Class

The main `ArticleWatcher` class extends the base `Watcher` class and handles:

- **Message Listening**: Responds to Chrome runtime messages
- **Article Detection**: Identifies when new articles are loaded
- **Asset Generation**: Extracts article metadata and content
- **Snapshot Creation**: Captures article state

### Message Types

- `NEW` - Triggered when a new article is detected
- `GENERATE_ASSETS` - Extracts article content and metadata
- `GENERATE_SNAPSHOT` - Creates a snapshot of the current article state

## Dependencies

### Runtime Dependencies

- None (bundled as standalone content script)

### Development Dependencies

- `@eurora/chrome-ext-shared` - Shared extension utilities
- `@eurora/shared` - Shared application utilities
- `vite` - Build tool
- `vitest` - Test framework
- `typescript` - Type checking
- `eslint` - Code linting
- `prettier` - Code formatting

## Chrome Extension Integration

This content script is injected into article pages and communicates with the background script via Chrome's messaging API.

### Permissions Required

- `activeTab` - Access to the current tab
- Host permissions for article domains

## Contributing

1. Follow the existing code style
2. Write tests for new features
3. Ensure all tests pass before submitting
4. Run linting and formatting

## License

See the main project LICENSE file.
