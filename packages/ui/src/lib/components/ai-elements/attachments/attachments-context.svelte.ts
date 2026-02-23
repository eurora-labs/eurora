import { getContext, setContext } from 'svelte';

export type AttachmentMediaCategory = 'image' | 'video' | 'audio' | 'document' | 'source' | 'unknown';

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

class AttachmentsState {
	variant = $state<AttachmentVariant>('grid');

	constructor(variant: AttachmentVariant) {
		this.variant = variant;
	}
}

class AttachmentItemState {
	data = $state<AttachmentData>({} as AttachmentData);
	mediaCategory = $state<AttachmentMediaCategory>('unknown');
	onRemove = $state<(() => void) | undefined>(undefined);
	variant = $state<AttachmentVariant>('grid');

	constructor(data: AttachmentData, variant: AttachmentVariant, onRemove?: () => void) {
		this.data = data;
		this.mediaCategory = getMediaCategory(data);
		this.onRemove = onRemove;
		this.variant = variant;
	}
}

const ATTACHMENTS_KEY = 'ai-attachments';
const ATTACHMENT_ITEM_KEY = 'ai-attachment-item';

export function setAttachmentsContext(variant: AttachmentVariant): AttachmentsState {
	const state = new AttachmentsState(variant);
	setContext(Symbol.for(ATTACHMENTS_KEY), state);
	return state;
}

export function getAttachmentsContext(): AttachmentsState {
	return getContext<AttachmentsState>(Symbol.for(ATTACHMENTS_KEY)) ?? new AttachmentsState('grid');
}

export function setAttachmentItemContext(
	data: AttachmentData,
	variant: AttachmentVariant,
	onRemove?: () => void,
): AttachmentItemState {
	const state = new AttachmentItemState(data, variant, onRemove);
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
