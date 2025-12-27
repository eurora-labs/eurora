
import Content from '$lib/components/select/select-content.svelte';
import Group from '$lib/components/select/select-group.svelte';
import Item from '$lib/components/select/select-item.svelte';
import Label from '$lib/components/select/select-label.svelte';
import ScrollDownButton from '$lib/components/select/select-scroll-down-button.svelte';
import ScrollUpButton from '$lib/components/select/select-scroll-up-button.svelte';
import Separator from '$lib/components/select/select-separator.svelte';
import Trigger from '$lib/components/select/select-trigger.svelte';
import { Select as SelectPrimitive } from 'bits-ui';

const Root = SelectPrimitive.Root;

export {
	Root,
	Group,
	Label,
	Item,
	Content,
	Trigger,
	Separator,
	ScrollDownButton,
	ScrollUpButton,
	//
	Root as Select,
	Group as SelectGroup,
	Label as SelectLabel,
	Item as SelectItem,
	Content as SelectContent,
	Trigger as SelectTrigger,
	Separator as SelectSeparator,
	ScrollDownButton as SelectScrollDownButton,
	ScrollUpButton as SelectScrollUpButton,
};
