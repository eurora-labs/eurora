import { getDocTitle, getResourceId, requireDocKind, siteName, type GoogleDocKind } from './_lib';
import { READABILITY_BODY_CAP, clampString } from '../../extensions/web/truncation';
import browser from 'webextension-polyfill';
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
	text_content: z.string(),
	excerpt: z.string(),
	length: z.number(),
	truncated: z.boolean(),
});

type Result = z.infer<typeof Out>;

const EXCERPT_LEN = 200;

function exportFormat(kind: GoogleDocKind): 'txt' | 'csv' {
	return kind === 'spreadsheets' ? 'csv' : 'txt';
}

interface FetchUrlOk {
	ok: true;
	text: string;
}
interface FetchUrlErr {
	ok: false;
	error: string;
}
type FetchUrlResponse = FetchUrlOk | FetchUrlErr;

function isFetchUrlResponse(value: unknown): value is FetchUrlResponse {
	if (typeof value !== 'object' || value === null) return false;
	const candidate = value as { ok?: unknown; text?: unknown; error?: unknown };
	if (candidate.ok === true) return typeof candidate.text === 'string';
	if (candidate.ok === false) return typeof candidate.error === 'string';
	return false;
}

/// Send the export URL to the background script's `FETCH_URL` handler.
/// Routing through the background is mandatory because MV3 content-script
/// `fetch()` calls don't carry the user's docs.google.com session cookies
/// (the request is treated as cross-origin from the extension's
/// perspective). The background context has `<all_urls>` host permission
/// and the right cookie handling.
async function fetchExport(
	kind: GoogleDocKind,
	resourceId: string,
	signal: AbortSignal,
): Promise<string> {
	if (signal.aborted) throw new DOMException('aborted', 'AbortError');
	const url = `https://docs.google.com/${kind}/d/${resourceId}/export?format=${exportFormat(kind)}`;
	const response = await new Promise<unknown>((resolve, reject) => {
		function onAbort() {
			reject(new DOMException('aborted', 'AbortError'));
		}
		signal.addEventListener('abort', onAbort, { once: true });
		browser.runtime
			.sendMessage({ type: 'FETCH_URL', url })
			.then((value) => {
				signal.removeEventListener('abort', onAbort);
				resolve(value);
			})
			.catch((err) => {
				signal.removeEventListener('abort', onAbort);
				reject(err);
			});
	});
	if (!isFetchUrlResponse(response)) {
		throw new Error('unexpected FETCH_URL response from background');
	}
	if (!response.ok) {
		throw new Error(`Google Docs export failed: ${response.error}`);
	}
	return response.text;
}

/// Fetch the open document's body via the background `FETCH_URL` relay,
/// which carries the user's docs.google.com session cookies. The response
/// is plain text for documents and CSV for spreadsheets; `text_content`
/// is clamped to `READABILITY_BODY_CAP` and `length` reports the
/// pre-truncation character count so the agent can detect elision.
export async function executeGetDocument(signal: AbortSignal): Promise<Result> {
	const kind = requireDocKind();
	const resourceId = getResourceId(kind);
	const raw = await fetchExport(kind, resourceId, signal);
	const clamped = clampString(raw, READABILITY_BODY_CAP);
	return {
		kind,
		resource_id: resourceId,
		title: getDocTitle(),
		site_name: siteName(kind),
		url: window.location.href,
		language: document.documentElement.lang || null,
		text_content: clamped.value,
		excerpt: clamped.value.slice(0, EXCERPT_LEN),
		length: raw.length,
		truncated: clamped.truncated,
	};
}

export const getDocument: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'google_docs_get_document',
		description:
			'Fetch the full body of the open Google Doc or Sheet via its export endpoint and return the plain text (Docs) or CSV (Sheets) content along with identifying metadata. The body is truncated to ~32 KiB; `length` reports the pre-truncation character count and `truncated` flags whether content was elided. Fails when the page is not a document or spreadsheet, the user is signed out, or the export endpoint is unreachable.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 10_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args, signal) {
		return await executeGetDocument(signal);
	},
};
