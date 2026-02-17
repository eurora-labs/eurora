/** Mirrors the NativeResponse type from the browser extension. */
export interface NativeResponse {
	kind: string;
	data: any;
}

export type WatcherResponse = NativeResponse | void;
