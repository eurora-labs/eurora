# @eurora/chrome-ext-shared

Shared utilities and types for Chrome extension development in the Eurora project.

## Overview

This package provides shared TypeScript utilities, type definitions, and helper functions for building Chrome extensions. It includes:

- Type bindings for native messaging
- Article extraction utilities using Mozilla Readability
- Watcher base class for content script observers
- Common models and interfaces

## Building

To build the library:

```bash
pnpm build
```

or

```bash
pnpm package
```

## Development

To run in development mode:

```bash
pnpm dev
```

## Testing

To run tests:

```bash
pnpm test
```

To run tests in watch mode:

```bash
pnpm test:unit
```

## Exports

The package exports the following modules:

- Type bindings (`NativeArticleAsset`, `NativeYoutubeAsset`, etc.)
- Models (`NativeResponse`)
- Article utilities (`createArticleAsset`, `createArticleSnapshot`)
- Watcher base class and types

## Usage

```typescript
import { createArticleAsset, Watcher, type NativeResponse } from '@eurora/chrome-ext-shared';
```
