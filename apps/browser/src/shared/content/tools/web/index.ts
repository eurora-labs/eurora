import type { z } from 'zod';
import { getAccessibilityTree } from './get_accessibility_tree';
import { getPageMetadata } from './get_page_metadata';
import { getReadabilityArticle } from './get_readability_article';
import { getSelectedText } from './get_selected_text';
import { insertText } from './insert_text';
import { listFormInputs } from './list_form_inputs';
import { listLinks } from './list_links';
import { querySelector } from './query_selector';
import type { Tool } from '../types';

export {
	getAccessibilityTree,
	getPageMetadata,
	getReadabilityArticle,
	getSelectedText,
	insertText,
	listFormInputs,
	listLinks,
	querySelector,
};

/// Default surface of generic web tools: read-only page primitives plus
/// the single mutating `insert_text`. Watchers compose this array
/// directly (`[...webTools, ...siteSpecific]`) — order doesn't matter
/// for dispatch, but stable order makes the `LIST_TOOLS` response
/// deterministic for the LLM's tool-selection heuristics.
export const webTools: readonly Tool<z.ZodTypeAny, unknown>[] = [
	getPageMetadata,
	getSelectedText,
	getReadabilityArticle,
	getAccessibilityTree,
	querySelector,
	listLinks,
	listFormInputs,
	insertText,
] as const;
