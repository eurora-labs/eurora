import {
	collectIconCandidatesFromLinks,
	originFallbackCandidate,
	resolveBestCandidate,
	type IconCandidate,
	type IconLinkRecord,
} from '../../../shared/background/favicon-ranker';
import browser from 'webextension-polyfill';

interface NativeMetadataPayload {
	kind: 'NativeMetadata';
	data: {
		url: string;
		icon_base64: string;
		title: string | null;
	};
}

interface ErrorPayload {
	kind: 'Error';
	data: string;
}

type CommonMessage = { type: 'GET_METADATA' };

function isCommonMessage(value: unknown): value is CommonMessage {
	return (
		typeof value === 'object' &&
		value !== null &&
		(value as { type?: unknown }).type === 'GET_METADATA'
	);
}

/// Page-metadata content-script handler. Separate from the tool
/// framework because `GET_METADATA` answers the bridge's
/// "what tab is the user looking at" probe, not a model-issued tool
/// call — it's plumbing that always answers regardless of the active
/// site bundle.
async function handleGetMetadata(): Promise<NativeMetadataPayload | ErrorPayload> {
	try {
		const icon_base64 = await resolveDocumentFavicon();
		return {
			kind: 'NativeMetadata',
			data: {
				url: window.location.href,
				icon_base64,
				title: document.title || null,
			},
		};
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error);
		console.error('Common watcher failed', { error });
		return { kind: 'Error', data: message };
	}
}

async function resolveDocumentFavicon(): Promise<string> {
	const records: IconLinkRecord[] = Array.from(
		document.querySelectorAll<HTMLLinkElement>('link[rel]'),
	)
		.filter((link) => !!link.href)
		.map((link) => ({
			href: link.href,
			rel: link.rel || '',
			type: link.type || '',
			sizes: link.getAttribute('sizes') || '',
		}));

	const candidates: IconCandidate[] = collectIconCandidatesFromLinks(records);

	const fallback = originFallbackCandidate(window.location.href, candidates.length);
	if (fallback) candidates.push(fallback);

	return await resolveBestCandidate(candidates);
}

let initialized = false;

export function main() {
	if (initialized) return;
	initialized = true;

	browser.runtime.onMessage.addListener(async (message) => {
		if (!isCommonMessage(message)) return undefined;
		return await handleGetMetadata();
	});
}
