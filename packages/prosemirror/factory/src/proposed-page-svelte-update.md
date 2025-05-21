# Proposed Update for +page.svelte

This file contains the proposed change for integrating the ExtensionFactory in `apps/desktop/src/routes/(launcher)/+page.svelte`.

## Current Implementation

```svelte
<script lang="ts">
  // ... existing imports
  import { transcriptExtension } from '@eurora/ext-transcript';
  import { videoExtension } from '@eurora/ext-video';
  
  // ... other code

  // Query object for the Launcher.Input component
  let searchQuery = $state({
    text: '',
    extensions: [transcriptExtension(), videoExtension()]
  });
  
  // ... rest of the file
</script>
```

## Proposed Implementation

```svelte
<script lang="ts">
  // ... existing imports
  
  // Replace individual extension imports with the factory import
  import { extensionFactory } from '@eurora/prosemirror-factory';
  // Import to ensure extensions are registered
  import '@eurora/prosemirror-factory/register-extensions';
  
  // ... other code

  // Query object for the Launcher.Input component using the factory
  let searchQuery = $state({
    text: '',
    extensions: extensionFactory.getExtensions()
  });
  
  // ... rest of the file
</script>
```

## Alternative Implementation (with specific extensions)

If you need to use specific extensions only:

```svelte
<script lang="ts">
  // ... existing imports
  
  // Replace individual extension imports with the factory import
  import { extensionFactory } from '@eurora/prosemirror-factory';
  // Import to ensure extensions are registered
  import '@eurora/prosemirror-factory/register-extensions';
  
  // Define constants for extension IDs
  const VIDEO_EXTENSION_ID = '9370B14D-B61C-4CE2-BDE7-B18684E8731A';
  const TRANSCRIPT_EXTENSION_ID = 'D8215655-A880-4B0F-8EFA-0B6B447F8AF3';
  
  // ... other code

  // Query object for the Launcher.Input component using specific extensions
  let searchQuery = $state({
    text: '',
    extensions: [
      extensionFactory.getExtension(VIDEO_EXTENSION_ID),
      extensionFactory.getExtension(TRANSCRIPT_EXTENSION_ID)
    ].filter(Boolean) // Filter out any undefined extensions
  });
  
  // ... rest of the file
</script>
```

## Benefits of This Approach

1. **Decoupling**: The page component is decoupled from specific extension implementations
2. **Flexibility**: New extensions can be added without changing the page component
3. **Centralized Management**: All extensions are managed through the factory
4. **Extensibility**: Additional extensions can be dynamically registered and used