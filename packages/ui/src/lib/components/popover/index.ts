import Content from '$lib/components/popover/popover-content.svelte';
import Trigger from '$lib/components/popover/popover-trigger.svelte';
import { Popover as PopoverPrimitive } from 'bits-ui';
const Root = PopoverPrimitive.Root;
const Close = PopoverPrimitive.Close;

export {
	Root,
	Content,
	Trigger,
	Close,
	//
	Root as Popover,
	Content as PopoverContent,
	Trigger as PopoverTrigger,
	Close as PopoverClose,
};
