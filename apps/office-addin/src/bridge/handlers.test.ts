import { ERROR_HANDLER_FAILED, ERROR_UNKNOWN_ACTION, dispatchRequest } from '$lib/bridge/handlers';
import { describe, expect, it, vi } from 'vitest';
import type { RequestFrame, WordDocumentAsset } from '$lib/shared/bindings';
import type { DocumentMetadata } from '$lib/word/extract';

const ASSET: WordDocumentAsset = { document_name: 'Doc', text: 'hello' };
const METADATA: DocumentMetadata = {
	title: 'Doc',
	author: 'Andre',
	last_modified: null,
	word_count: 1,
};

function req(action: string): RequestFrame {
	return { id: 1, action, payload: null };
}

describe('dispatchRequest', () => {
	it('returns a Response with the JSON-encoded WordDocumentAsset for GET_ASSETS', async () => {
		const deps = {
			getAsset: vi.fn().mockResolvedValue(ASSET),
			getMetadata: vi.fn(),
		};
		const frame = await dispatchRequest(req('GET_ASSETS'), deps);
		expect(deps.getAsset).toHaveBeenCalledOnce();
		expect(frame).toEqual({
			kind: { Response: { id: 1, action: 'GET_ASSETS', payload: JSON.stringify(ASSET) } },
		});
	});

	it('returns a Response with the JSON-encoded metadata for GET_METADATA', async () => {
		const deps = {
			getAsset: vi.fn(),
			getMetadata: vi.fn().mockResolvedValue(METADATA),
		};
		const frame = await dispatchRequest(req('GET_METADATA'), deps);
		expect(deps.getMetadata).toHaveBeenCalledOnce();
		expect(frame).toEqual({
			kind: {
				Response: { id: 1, action: 'GET_METADATA', payload: JSON.stringify(METADATA) },
			},
		});
	});

	it('returns an Error frame for an unknown action', async () => {
		const frame = await dispatchRequest(req('GET_UNICORNS'), {
			getAsset: vi.fn(),
			getMetadata: vi.fn(),
		});
		expect(frame.kind).toHaveProperty('Error');
		const err = (frame.kind as { Error: { code: number; message: string } }).Error;
		expect(err.code).toBe(ERROR_UNKNOWN_ACTION);
		expect(err.message).toContain('GET_UNICORNS');
	});

	it('wraps thrown handler errors in an Error frame', async () => {
		const frame = await dispatchRequest(req('GET_ASSETS'), {
			getAsset: vi.fn().mockRejectedValue(new Error('Word said no')),
			getMetadata: vi.fn(),
		});
		expect(frame.kind).toHaveProperty('Error');
		const err = (frame.kind as { Error: { code: number; message: string } }).Error;
		expect(err.code).toBe(ERROR_HANDLER_FAILED);
		expect(err.message).toBe('Word said no');
	});
});
