/**
 * Integration Examples for Page Components
 *
 * This file contains examples showing how to integrate the extension factory
 * with page components. These are NOT meant to be compiled directly - they are
 * example code snippets for documentation purposes.
 *
 * Original code in apps/desktop/src/routes/(launcher)/+page.svelte:
 *
 * ```typescript
 * import { transcriptExtension } from '@eurora/ext-transcript';
 * import { videoExtension } from '@eurora/ext-video';
 *
 * // Query object for the Launcher.Input component
 * let searchQuery = $state({
 *   text: '',
 *   extensions: [transcriptExtension(), videoExtension()]
 * });
 * ```
 */

// DO NOT USE THIS FILE DIRECTLY - THESE ARE EXAMPLE SNIPPETS

/*
 * Example 1: Get all available extensions
 *
 * ```typescript
 * // Import the factory instead of individual extensions
 * import { extensionFactory } from '@eurora/prosemirror-factory';
 * // Import this to ensure extensions are registered
 * import '@eurora/prosemirror-factory/register-extensions';
 *
 * // Query object for the Launcher.Input component
 * let searchQuery = $state({
 *   text: '',
 *   extensions: extensionFactory.getExtensions()
 * });
 * ```
 */

/*
 * Example 2: Get specific extensions by ID
 *
 * ```typescript
 * import { extensionFactory } from '@eurora/prosemirror-factory';
 * import '@eurora/prosemirror-factory/register-extensions';
 *
 * // Define the IDs of the extensions you want to use
 * const EXTENSION_IDS = {
 *   VIDEO: '9370B14D-B61C-4CE2-BDE7-B18684E8731A',
 *   TRANSCRIPT: 'D8215655-A880-4B0F-8EFA-0B6B447F8AF3'
 * };
 *
 * // Query object for the Launcher.Input component
 * let searchQuery = $state({
 *   text: '',
 *   extensions: [
 *     extensionFactory.getExtension(EXTENSION_IDS.VIDEO),
 *     extensionFactory.getExtension(EXTENSION_IDS.TRANSCRIPT)
 *   ].filter(Boolean) // Filter out any undefined values
 * });
 * ```
 */

/*
 * Example 3: Using utility functions
 *
 * ```typescript
 * import { extensionFactory } from '@eurora/prosemirror-factory';
 * import { getExtensionsByNamePattern } from '@eurora/prosemirror-factory/utils';
 * import '@eurora/prosemirror-factory/register-extensions';
 *
 * // Query object for the Launcher.Input component
 * let searchQuery = $state({
 *   text: '',
 *   // Get only media-related extensions
 *   extensions: getExtensionsByNamePattern(/video|media/i)
 * });
 * ```
 */

/*
 * Example 4: Conditionally include extensions
 *
 * ```typescript
 * import { extensionFactory } from '@eurora/prosemirror-factory';
 * import '@eurora/prosemirror-factory/register-extensions';
 *
 * // Feature flags for extensions
 * const enableVideoExtension = true;
 * const enableTranscriptExtension = true;
 *
 * // Build the list of extensions based on feature flags
 * const activeExtensions = [];
 *
 * if (enableVideoExtension) {
 *   const videoExt = extensionFactory.getExtension('9370B14D-B61C-4CE2-BDE7-B18684E8731A');
 *   if (videoExt) activeExtensions.push(videoExt);
 * }
 *
 * if (enableTranscriptExtension) {
 *   const transcriptExt = extensionFactory.getExtension('D8215655-A880-4B0F-8EFA-0B6B447F8AF3');
 *   if (transcriptExt) activeExtensions.push(transcriptExt);
 * }
 *
 * // Query object for the Launcher.Input component
 * let searchQuery = $state({
 *   text: '',
 *   extensions: activeExtensions
 * });
 * ```
 */

// This is a documentation file with code examples, not meant to be compiled
export {};
