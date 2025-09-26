import Root from './chat.svelte';

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

import Message, { type MessageProps } from './chat-message.svelte';
import Content from './chat-message-content.svelte';
import Source from './chat-message-source.svelte';
import Footer from './chat-message-footer.svelte';
