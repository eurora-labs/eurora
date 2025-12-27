import Content from '$lib/components/hover-card/hover-card-content.svelte';
import Trigger from '$lib/components/hover-card/hover-card-trigger.svelte';
import { LinkPreview as HoverCardPrimitive } from 'bits-ui';

const Root = HoverCardPrimitive.Root;

export {
	Root,
	Content,
	Trigger,
	Root as HoverCard,
	Content as HoverCardContent,
	Trigger as HoverCardTrigger,
};
