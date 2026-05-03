import { errorFrame, responseFrame } from '$lib/bridge/frames';
import * as log from '$lib/util/log';
import { getDocumentAsset, getDocumentMetadata } from '$lib/word/extract';
import type { Frame, RequestFrame } from '$lib/shared/bindings';

export type Action = 'GET_ASSETS' | 'GET_METADATA';

export interface HandlerDeps {
	getAsset: typeof getDocumentAsset;
	getMetadata: typeof getDocumentMetadata;
}

const PRODUCTION_DEPS: HandlerDeps = {
	getAsset: getDocumentAsset,
	getMetadata: getDocumentMetadata,
};

export async function dispatchRequest(
	req: RequestFrame,
	deps: HandlerDeps = PRODUCTION_DEPS,
): Promise<Frame> {
	try {
		switch (req.action) {
			case 'GET_ASSETS': {
				const asset = await deps.getAsset();
				return responseFrame(req.id, req.action, JSON.stringify(asset));
			}
			case 'GET_METADATA': {
				const metadata = await deps.getMetadata();
				return responseFrame(req.id, req.action, JSON.stringify(metadata));
			}
			default:
				log.warn('unknown action', req.action);
				return errorFrame(req.id, ERROR_UNKNOWN_ACTION, `Unknown action: ${req.action}`);
		}
	} catch (e) {
		const message = e instanceof Error ? e.message : String(e);
		log.error('handler threw', req.action, e);
		return errorFrame(req.id, ERROR_HANDLER_FAILED, message);
	}
}

export const ERROR_UNKNOWN_ACTION = 1;
export const ERROR_HANDLER_FAILED = 2;
