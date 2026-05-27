/// Public entry point for the content-script tool surface. Consumers
/// outside the extension (today: the `@eurora/e2e` Playwright suite)
/// import from `@eurora/browser/tools` to get the wire-protocol types and
/// the typed tool objects without reaching into deep relative paths that
/// would couple them to the internal directory layout.
///
/// The desktop side does NOT consume this barrel — it speaks to the
/// extension over the four-message bridge protocol declared in
/// `install.ts` (`LIST_TOOLS` / `GET_CONTEXT` / `INVOKE_TOOL` /
/// `CANCEL_TOOL`) and decodes the wire descriptors itself.

export type {
	InvokeResponse,
	JsonSchemaFragment,
	ListToolsResponse,
	ToolErrorWire,
	ToolSource,
	WireToolDescriptor,
} from './wire';
export type { ContextResponse, Tool, Watcher } from './types';

export * as twitterTools from './twitter';
export * as webToolset from './web';
export * as youtubeTools from './youtube';
export * as googleDocsToolset from './google_docs';
