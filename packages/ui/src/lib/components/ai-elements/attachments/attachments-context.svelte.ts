import { getContext, setContext } from 'svelte';

export type AttachmentMediaCategory =
	| 'image'
	| 'video'
	| 'audio'
	| 'document'
	| 'source'
	| 'unknown';

export type AttachmentVariant = 'grid' | 'inline' | 'list';

export interface FileAttachmentData {
	type: 'file';
	id: string;
	url?: string;
	mediaType?: string;
	filename?: string;
}

export interface SourceDocumentAttachmentData {
	type: 'source-document';
	id: string;
	title?: string;
	filename?: string;
	url?: string;
	mediaType?: string;
}

export type AttachmentData = FileAttachmentData | SourceDocumentAttachmentData;

export function getMediaCategory(data: AttachmentData): AttachmentMediaCategory {
	if (data.type === 'source-document') {
		return 'source';
	}

	const mediaType = data.mediaType ?? '';

	if (mediaType.startsWith('image/')) return 'image';
	if (mediaType.startsWith('video/')) return 'video';
	if (mediaType.startsWith('audio/')) return 'audio';
	if (mediaType.startsWith('application/') || mediaType.startsWith('text/')) return 'document';

	return 'unknown';
}

export function getAttachmentLabel(data: AttachmentData): string {
	if (data.type === 'source-document') {
		return data.title || data.filename || 'Source';
	}

	const category = getMediaCategory(data);
	return data.filename || (category === 'image' ? 'Image' : 'Attachment');
}

export interface AttachmentsStateOptions {
	variant: () => AttachmentVariant;
}

class AttachmentsState {
	readonly #opts: AttachmentsStateOptions;

	constructor(opts: AttachmentsStateOptions) {
		this.#opts = opts;
	}

	get variant(): AttachmentVariant {
		return this.#opts.variant();
	}
}

export interface AttachmentItemStateOptions {
	data: () => AttachmentData;
	variant: () => AttachmentVariant;
	onRemove?: () => (() => void) | undefined;
}

class AttachmentItemState {
	readonly #opts: AttachmentItemStateOptions;

	constructor(opts: AttachmentItemStateOptions) {
		this.#opts = opts;
	}

	get data(): AttachmentData {
		return this.#opts.data();
	}

	get variant(): AttachmentVariant {
		return this.#opts.variant();
	}

	get onRemove(): (() => void) | undefined {
		return this.#opts.onRemove?.();
	}

	get mediaCategory(): AttachmentMediaCategory {
		return getMediaCategory(this.data);
	}
}

const ATTACHMENTS_KEY = 'ai-attachments';
const ATTACHMENT_ITEM_KEY = 'ai-attachment-item';

export function setAttachmentsContext(opts: AttachmentsStateOptions): AttachmentsState {
	const state = new AttachmentsState(opts);
	setContext(Symbol.for(ATTACHMENTS_KEY), state);
	return state;
}

export function getAttachmentsContext(): AttachmentsState {
	return (
		getContext<AttachmentsState>(Symbol.for(ATTACHMENTS_KEY)) ??
		new AttachmentsState({ variant: () => 'grid' })
	);
}

export function setAttachmentItemContext(opts: AttachmentItemStateOptions): AttachmentItemState {
	const state = new AttachmentItemState(opts);
	setContext(Symbol.for(ATTACHMENT_ITEM_KEY), state);
	return state;
}

export function getAttachmentItemContext(): AttachmentItemState {
	const context = getContext<AttachmentItemState>(Symbol.for(ATTACHMENT_ITEM_KEY));
	if (!context) {
		throw new Error('Attachment components must be used within <Attachment>');
	}
	return context;
}
