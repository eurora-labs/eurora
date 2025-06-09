# ProseMirror Extension Factory

A factory system for managing and providing ProseMirror extensions in Eurora.

## Overview

This package provides a centralized system for registering, managing, and retrieving ProseMirror extensions through a factory pattern. It enables applications to:

- Manage extensions through a single registry
- Access extensions by their unique IDs
- Get all available extensions at once
- Group and filter extensions based on various criteria

## Installation

```bash
# Using npm
npm install @eurora/prosemirror-factory

# Using pnpm
pnpm add @eurora/prosemirror-factory
```

## Usage

### Basic Usage

```typescript
import { extensionFactory } from '@eurora/prosemirror-factory';
import { registerCoreExtensions } from '@eurora/prosemirror-factory/register-extensions';

// Register built-in extensions
registerCoreExtensions();

// Get all registered extensions
const allExtensions = extensionFactory.getExtensions();

// Get a specific extension by ID
const videoExtension = extensionFactory.getExtension('9370B14D-B61C-4CE2-BDE7-B18684E8731A');
```

### Integration with Launcher

To use this in the `apps/desktop/src/routes/(launcher)/+page.svelte` file:

```typescript
import { extensionFactory } from '@eurora/prosemirror-factory';
// Import to ensure extensions are registered
import '@eurora/prosemirror-factory/register-extensions';

// In your component
let searchQuery = $state({
	text: '',
	extensions: extensionFactory.getExtensions(),
});
```

### Utility Functions

The package provides several utility functions to work with extensions:

```typescript
import {
	getExtensionsByNamePattern,
	getExtensionsByNodeType,
	createSchemaConfig,
	groupExtensionsByCategory,
	hasRequiredExtensions,
} from '@eurora/prosemirror-factory/utils';

// Get extensions by pattern
const mediaExtensions = getExtensionsByNamePattern(/video|audio/i);

// Check if required extensions are available
const hasRequired = hasRequiredExtensions([
	'9370B14D-B61C-4CE2-BDE7-B18684E8731A', // video
	'D8215655-A880-4B0F-8EFA-0B6B447F8AF3', // transcript
]);
```

## Registering Custom Extensions

You can register your own extensions with the factory:

```typescript
import { extensionFactory } from '@eurora/prosemirror-factory';
import { myCustomExtension } from './my-extension';

// Register a custom extension
const ext = myCustomExtension();
extensionFactory.register(ext.id, myCustomExtension);
```

## Advanced Usage

See the examples in the `example-usage.ts` file for advanced usage patterns.

## Extension Architecture

Extensions should follow this structure to work with the factory:

```typescript
export const ID = 'A-UNIQUE-UUID';

export function myExtension() {
	return {
		id: ID,
		name: 'my-extension',
		// Other extension properties...
	};
}
```

## API Reference

### ExtensionFactory

- `register(id: string, creator: ExtensionCreator)`: Register an extension creator by ID
- `getExtension(id: string)`: Get an extension instance by ID
- `getExtensions()`: Get all registered extensions as instances
- `hasExtension(id: string)`: Check if an extension is registered
- `getExtensionIds()`: Get the IDs of all registered extensions
- `unregister(id: string)`: Unregister an extension by ID

### Utility Functions

- `getExtensionsByNamePattern(pattern: RegExp)`: Get extensions by name pattern
- `getExtensionsByNodeType(nodeType: string)`: Get extensions by node type
- `createSchemaConfig(extensions: SveltePMExtension[])`: Create schema config from extensions
- `groupExtensionsByCategory()`: Group extensions by category
- `hasRequiredExtensions(requiredIds: string[])`: Check if all required extensions are registered
