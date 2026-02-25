import Root from './attachments.svelte';
import Item from './attachment.svelte';
import Preview from './attachment-preview.svelte';
import Info from './attachment-info.svelte';
import Remove from './attachment-remove.svelte';
import HoverCard from './attachment-hover-card.svelte';
import HoverCardTrigger from './attachment-hover-card-trigger.svelte';
import HoverCardContent from './attachment-hover-card-content.svelte';
import Empty from './attachment-empty.svelte';

export {
	Root,
	Item,
	Preview,
	Info,
	Remove,
	HoverCard,
	HoverCardTrigger,
	HoverCardContent,
	Empty,
	//
	Root as Attachments,
	Item as Attachment,
	Preview as AttachmentPreview,
	Info as AttachmentInfo,
	Remove as AttachmentRemove,
	HoverCard as AttachmentHoverCard,
	HoverCardTrigger as AttachmentHoverCardTrigger,
	HoverCardContent as AttachmentHoverCardContent,
	Empty as AttachmentEmpty,
};

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
