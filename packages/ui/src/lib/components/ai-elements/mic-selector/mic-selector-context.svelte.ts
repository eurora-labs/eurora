import { getContext, setContext } from 'svelte';

const MIC_SELECTOR_CONTEXT_KEY = Symbol.for('mic-selector-context');

export interface MicSelectorContextOptions {
	devices: () => MediaDeviceInfo[];
	value: () => string | undefined;
	setValue: (value: string | undefined) => void;
	open: () => boolean;
	setOpen: (open: boolean) => void;
}

export class MicSelectorContext {
	readonly #opts: MicSelectorContextOptions;
	#width = $state(200);

	constructor(opts: MicSelectorContextOptions) {
		this.#opts = opts;
	}

	get devices(): MediaDeviceInfo[] {
		return this.#opts.devices();
	}

	get value(): string | undefined {
		return this.#opts.value();
	}

	set value(val: string | undefined) {
		this.#opts.setValue(val);
	}

	get open(): boolean {
		return this.#opts.open();
	}

	set open(val: boolean) {
		this.#opts.setOpen(val);
	}

	get width() {
		return this.#width;
	}

	set width(val: number) {
		this.#width = val;
	}
}

export function setMicSelectorContext(context: MicSelectorContext) {
	setContext(MIC_SELECTOR_CONTEXT_KEY, context);
}

export function getMicSelectorContext(): MicSelectorContext {
	const context = getContext<MicSelectorContext | undefined>(MIC_SELECTOR_CONTEXT_KEY);
	if (!context) {
		throw new Error('MicSelector components must be used within MicSelector');
	}
	return context;
}

const deviceIdRegex = /\(([\da-fA-F]{4}:[\da-fA-F]{4})\)$/;

export function parseDeviceLabel(label: string): { name: string; deviceId?: string } {
	const matches = label.match(deviceIdRegex);
	if (!matches) return { name: label };
	const [, id] = matches;
	return { name: label.replace(deviceIdRegex, ''), deviceId: id };
}

export function useAudioDevices() {
	let devices = $state<MediaDeviceInfo[]>([]);
	let loading = $state(true);
	let error = $state<string | null>(null);
	let hasPermission = $state(false);

	async function loadDevicesWithoutPermission() {
		try {
			loading = true;
			error = null;
			const deviceList = await navigator.mediaDevices.enumerateDevices();
			devices = deviceList.filter((device) => device.kind === 'audioinput');
		} catch (err) {
			const message = err instanceof Error ? err.message : 'Failed to get audio devices';
			error = message;
		} finally {
			loading = false;
		}
	}

	async function loadDevicesWithPermission() {
		if (loading) return;
		try {
			loading = true;
			error = null;
			const tempStream = await navigator.mediaDevices.getUserMedia({ audio: true });
			for (const track of tempStream.getTracks()) {
				track.stop();
			}
			const deviceList = await navigator.mediaDevices.enumerateDevices();
			devices = deviceList.filter((device) => device.kind === 'audioinput');
			hasPermission = true;
		} catch (err) {
			const message = err instanceof Error ? err.message : 'Failed to get audio devices';
			error = message;
		} finally {
			loading = false;
		}
	}

	let cleanup: (() => void) | undefined;

	if (typeof window !== 'undefined') {
		loadDevicesWithoutPermission();

		const handleDeviceChange = () => {
			if (hasPermission) {
				loadDevicesWithPermission();
			} else {
				loadDevicesWithoutPermission();
			}
		};

		navigator.mediaDevices.addEventListener('devicechange', handleDeviceChange);
		cleanup = () => {
			navigator.mediaDevices.removeEventListener('devicechange', handleDeviceChange);
		};
	}

	return {
		get devices() {
			return devices;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		get hasPermission() {
			return hasPermission;
		},
		loadDevices: loadDevicesWithPermission,
		destroy() {
			cleanup?.();
		},
	};
}
