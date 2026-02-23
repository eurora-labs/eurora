import { getContext, setContext } from 'svelte';
import { nanoid } from 'nanoid';

// ============================================================================
// Types
// ============================================================================

export interface FileUIPart {
	id: string;
	filename: string;
	mediaType: string;
	type: 'file';
	url: string;
}

export interface SourceDocumentUIPart {
	sourceDocument: {
		id: string;
		title?: string;
		url?: string;
		[key: string]: unknown;
	};
	type: 'source-document';
}

export type ChatStatus = 'ready' | 'submitted' | 'streaming' | 'error';

export interface PromptInputMessage {
	text: string;
	files: Omit<FileUIPart, 'id'>[];
}

// ============================================================================
// Attachments State
// ============================================================================

export class AttachmentsState {
	files = $state<FileUIPart[]>([]);
	#fileInputEl: HTMLInputElement | null = null;

	setFileInput(el: HTMLInputElement | null) {
		this.#fileInputEl = el;
	}

	add(incoming: File[] | FileList) {
		const arr = [...incoming];
		if (arr.length === 0) return;
		const newFiles: FileUIPart[] = arr.map((file) => ({
			filename: file.name,
			id: nanoid(),
			mediaType: file.type,
			type: 'file' as const,
			url: URL.createObjectURL(file),
		}));
		this.files = [...this.files, ...newFiles];
	}

	remove(id: string) {
		const found = this.files.find((f) => f.id === id);
		if (found?.url) {
			URL.revokeObjectURL(found.url);
		}
		this.files = this.files.filter((f) => f.id !== id);
	}

	clear() {
		for (const f of this.files) {
			if (f.url) {
				URL.revokeObjectURL(f.url);
			}
		}
		this.files = [];
	}

	openFileDialog() {
		this.#fileInputEl?.click();
	}

	destroy() {
		for (const f of this.files) {
			if (f.url) {
				URL.revokeObjectURL(f.url);
			}
		}
	}
}

// ============================================================================
// Text Input State
// ============================================================================

export class TextInputState {
	value = $state('');

	constructor(initialValue = '') {
		this.value = initialValue;
	}

	setInput(v: string) {
		this.value = v;
	}

	clear() {
		this.value = '';
	}
}

// ============================================================================
// Referenced Sources State
// ============================================================================

export class ReferencedSourcesState {
	sources = $state<(SourceDocumentUIPart & { id: string })[]>([]);

	add(incoming: SourceDocumentUIPart[] | SourceDocumentUIPart) {
		const array = Array.isArray(incoming) ? incoming : [incoming];
		this.sources = [...this.sources, ...array.map((s) => ({ ...s, id: nanoid() }))];
	}

	remove(id: string) {
		this.sources = this.sources.filter((s) => s.id !== id);
	}

	clear() {
		this.sources = [];
	}
}

// ============================================================================
// Controller State (for Provider mode)
// ============================================================================

export class PromptInputControllerState {
	textInput: TextInputState;
	attachments: AttachmentsState;
	#openCallback: (() => void) | null = null;

	constructor(initialInput = '') {
		this.textInput = new TextInputState(initialInput);
		this.attachments = new AttachmentsState();
	}

	registerFileInput(el: HTMLInputElement | null, open: () => void) {
		this.attachments.setFileInput(el);
		this.#openCallback = open;
	}

	openFileDialog() {
		this.#openCallback?.();
	}
}

// ============================================================================
// Context Keys & Accessors
// ============================================================================

const CONTROLLER_KEY = 'ai-prompt-input-controller';
const LOCAL_ATTACHMENTS_KEY = 'ai-prompt-input-local-attachments';
const PROVIDER_ATTACHMENTS_KEY = 'ai-prompt-input-provider-attachments';
const REFERENCED_SOURCES_KEY = 'ai-prompt-input-referenced-sources';

export function setPromptInputController(
	state: PromptInputControllerState,
): PromptInputControllerState {
	return setContext(Symbol.for(CONTROLLER_KEY), state);
}

export function usePromptInputController(): PromptInputControllerState {
	return getContext(Symbol.for(CONTROLLER_KEY));
}

export function useOptionalPromptInputController(): PromptInputControllerState | undefined {
	return getContext<PromptInputControllerState | undefined>(Symbol.for(CONTROLLER_KEY));
}

export function setLocalAttachments(state: AttachmentsState): AttachmentsState {
	return setContext(Symbol.for(LOCAL_ATTACHMENTS_KEY), state);
}

export function useLocalAttachments(): AttachmentsState | undefined {
	return getContext<AttachmentsState | undefined>(Symbol.for(LOCAL_ATTACHMENTS_KEY));
}

export function setProviderAttachments(state: AttachmentsState): AttachmentsState {
	return setContext(Symbol.for(PROVIDER_ATTACHMENTS_KEY), state);
}

export function useProviderAttachments(): AttachmentsState | undefined {
	return getContext<AttachmentsState | undefined>(Symbol.for(PROVIDER_ATTACHMENTS_KEY));
}

export function usePromptInputAttachments(): AttachmentsState {
	const local = useLocalAttachments();
	const provider = useProviderAttachments();
	const ctx = local ?? provider;
	if (!ctx) {
		throw new Error(
			'usePromptInputAttachments must be used within a PromptInput or PromptInputProvider',
		);
	}
	return ctx;
}

export function setReferencedSources(state: ReferencedSourcesState): ReferencedSourcesState {
	return setContext(Symbol.for(REFERENCED_SOURCES_KEY), state);
}

export function useReferencedSources(): ReferencedSourcesState {
	return getContext(Symbol.for(REFERENCED_SOURCES_KEY));
}
