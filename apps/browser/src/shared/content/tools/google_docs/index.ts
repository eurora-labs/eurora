import { getDocument } from './get_document';
import { getMetadata } from './get_metadata';
import type { Tool } from '../types';
import type { z } from 'zod';

export { getDocument, getMetadata };

/// Google Docs / Sheets tools surfaced in addition to the generic web
/// tools when the user is viewing an actual document or spreadsheet. The
/// docs.google.com watcher composes `[...webTools, ...googleDocsTools]`
/// only when `detectDocKind()` resolves; off-product pages (the file
/// picker, account settings) just see the generic web tools.
export const googleDocsTools: readonly Tool<z.ZodTypeAny, unknown>[] = [
	getMetadata,
	getDocument,
] as const;
