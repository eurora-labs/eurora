import '$lib/app.css';

import Empty from '$lib/launcher/command-empty.svelte';
import Group from '$lib/launcher/command-group.svelte';
import Input from '$lib/launcher/command-input.svelte';
import Item from '$lib/launcher/command-item.svelte';
import LinkItem from '$lib/launcher/command-link-item.svelte';
import List from '$lib/launcher/command-list.svelte';
import Separator from '$lib/launcher/command-separator.svelte';
import Shortcut from '$lib/launcher/command-shortcut.svelte';
import Root from '$lib/launcher/command.svelte';
import { Command as CommandPrimitive } from 'bits-ui';

const Loading: typeof CommandPrimitive.Loading = CommandPrimitive.Loading;

export {
	Root,
	Empty,
	Group,
	Item,
	LinkItem,
	Input,
	List,
	Separator,
	Shortcut,
	Loading,
	Root as Command,
	Empty as CommandEmpty,
	Group as CommandGroup,
	Item as CommandItem,
	LinkItem as CommandLinkItem,
	Input as CommandInput,
	List as CommandList,
	Separator as CommandSeparator,
	Shortcut as CommandShortcut,
	Loading as CommandLoading,
};
