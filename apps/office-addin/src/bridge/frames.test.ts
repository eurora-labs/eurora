import {
	errorFrame,
	isCancel,
	isError,
	isEvent,
	isRegister,
	isRequest,
	isResponse,
	parseFrame,
	registerFrame,
	responseFrame,
} from '$lib/bridge/frames';
import { describe, expect, it } from 'vitest';

describe('frame constructors', () => {
	it('builds a Register frame with the expected envelope', () => {
		const frame = registerFrame(0, 42, 'microsoft-word');
		expect(frame).toEqual({
			kind: { Register: { host_pid: 0, app_pid: 42, app_kind: 'microsoft-word' } },
		});
	});

	it('builds a Response frame with a string payload', () => {
		const frame = responseFrame(7, 'GET_ASSETS', '"hello"');
		expect(frame).toEqual({
			kind: { Response: { id: 7, action: 'GET_ASSETS', payload: '"hello"' } },
		});
	});

	it('builds a Response frame with a null payload', () => {
		const frame = responseFrame(7, 'GET_ASSETS', null);
		expect(frame.kind).toHaveProperty('Response.payload', null);
	});

	it('builds an Error frame with default null details', () => {
		const frame = errorFrame(9, 1, 'boom');
		expect(frame).toEqual({
			kind: { Error: { id: 9, code: 1, message: 'boom', details: null } },
		});
	});
});

describe('frame type guards', () => {
	const cases = [
		{ guard: isRegister, kind: { Register: { host_pid: 0, app_pid: 1, app_kind: null } } },
		{ guard: isRequest, kind: { Request: { id: 1, action: 'X', payload: null } } },
		{ guard: isResponse, kind: { Response: { id: 1, action: 'X', payload: null } } },
		{ guard: isEvent, kind: { Event: { action: 'X', payload: null } } },
		{ guard: isError, kind: { Error: { id: 1, code: 0, message: 'm', details: null } } },
		{ guard: isCancel, kind: { Cancel: { id: 1 } } },
	];

	for (const { guard, kind } of cases) {
		it(`${guard.name} matches its variant`, () => {
			expect(guard(kind as never)).toBe(true);
		});
	}

	it('guards return false for foreign variants', () => {
		const reg = { Register: { host_pid: 0, app_pid: 0, app_kind: null } };
		expect(isRequest(reg as never)).toBe(false);
	});
});

describe('parseFrame', () => {
	it('accepts a well-formed frame', () => {
		const frame = parseFrame({
			kind: { Request: { id: 1, action: 'GET_ASSETS', payload: null } },
		});
		expect(frame).not.toBeNull();
	});

	it('rejects null and primitives', () => {
		expect(parseFrame(null)).toBeNull();
		expect(parseFrame(42)).toBeNull();
		expect(parseFrame('frame')).toBeNull();
	});

	it('rejects objects without a kind discriminator', () => {
		expect(parseFrame({})).toBeNull();
		expect(parseFrame({ kind: null })).toBeNull();
		expect(parseFrame({ kind: 'Request' })).toBeNull();
	});

	it('rejects unknown discriminator tags', () => {
		expect(parseFrame({ kind: { Bogus: {} } })).toBeNull();
	});

	it('rejects multi-tag kinds', () => {
		expect(
			parseFrame({
				kind: {
					Request: { id: 1, action: 'X', payload: null },
					Response: { id: 1, action: 'X', payload: null },
				},
			}),
		).toBeNull();
	});
});
