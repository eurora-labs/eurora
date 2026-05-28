import { getDocTitle, getResourceId, requireDocKind, siteName } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	kind: z.enum(['document', 'spreadsheets']),
	resource_id: z.string(),
	title: z.string(),
	site_name: z.enum(['Google Docs', 'Google Sheets']),
	url: z.string(),
	language: z.string().nullable(),
});

type Result = z.infer<typeof Out>;

export async function executeGetMetadata(): Promise<Result> {
	const kind = requireDocKind();
	return {
		kind,
		resource_id: getResourceId(kind),
		title: getDocTitle(),
		site_name: siteName(kind),
		url: window.location.href,
		language: document.documentElement.lang || null,
	};
}

export const getMetadata: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'google_docs_get_metadata',
		description:
			'Return identifying metadata for the open Google Docs / Sheets resource: product kind, resource id, title, URL, and page language. No network round-trip — useful as a cheap probe before deciding whether to fetch the body via `google_docs_get_document`. Fails when the page is not a document or spreadsheet (e.g. the file picker).',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 2_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeGetMetadata();
	},
};
