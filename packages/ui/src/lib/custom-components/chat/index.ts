import Content from '$lib/custom-components/chat/chat-message-content.svelte';
import Footer from '$lib/custom-components/chat/chat-message-footer.svelte';
import Source from '$lib/custom-components/chat/chat-message-source.svelte';
import Message, { type MessageProps } from '$lib/custom-components/chat/chat-message.svelte';
import Root from '$lib/custom-components/chat/chat.svelte';

export {
	Root,
	//
	Root as Chat,
	Message,
	Content,
	Source,
	Footer,
	//
	type MessageProps,
	//
	Content as MessageContent,
	Source as MessageSource,
	Footer as MessageFooter,
};
