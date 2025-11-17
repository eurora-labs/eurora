import type { FrameKind, FrameEndpoint } from './bindings.js';

export const FrameKindToIndex: Record<FrameKind, number> = {
	Unspecified: 0,
	Request: 1,
	Response: 2,
	Event: 3,
	Error: 4,
	Cancel: 5,
};

export const FrameEndpointToIndex: Record<FrameEndpoint, number> = {
	Unspecified: 0,
	Browser: 1,
	Tauri: 2,
};
