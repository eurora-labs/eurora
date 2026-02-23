export { default as Attachments } from './attachments.svelte';
export { default as Attachment } from './attachment.svelte';
export { default as AttachmentPreview } from './attachment-preview.svelte';
export { default as AttachmentInfo } from './attachment-info.svelte';
export { default as AttachmentRemove } from './attachment-remove.svelte';
export { default as AttachmentHoverCard } from './attachment-hover-card.svelte';
export { default as AttachmentHoverCardTrigger } from './attachment-hover-card-trigger.svelte';
export { default as AttachmentHoverCardContent } from './attachment-hover-card-content.svelte';
export { default as AttachmentEmpty } from './attachment-empty.svelte';

export {
	getMediaCategory,
	getAttachmentLabel,
	getAttachmentsContext,
	setAttachmentsContext,
	getAttachmentItemContext,
	setAttachmentItemContext,
	type AttachmentData,
	type FileAttachmentData,
	type SourceDocumentAttachmentData,
	type AttachmentMediaCategory,
	type AttachmentVariant,
} from './attachments-context.svelte.js';
