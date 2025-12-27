
import Dialog from '$lib/components/command/command-dialog.svelte';
import Empty from '$lib/components/command/command-empty.svelte';
import Group from '$lib/components/command/command-group.svelte';
import Input from '$lib/components/command/command-input.svelte';
import Item from '$lib/components/command/command-item.svelte';
import LinkItem from '$lib/components/command/command-link-item.svelte';
import List from '$lib/components/command/command-list.svelte';
import Separator from '$lib/components/command/command-separator.svelte';
import Shortcut from '$lib/components/command/command-shortcut.svelte';
import Root from '$lib/components/command/command.svelte';
import { Command as CommandPrimitive } from 'bits-ui';

const Loading = CommandPrimitive.Loading;

export {
	Root,
	Dialog,
	Empty,
	Group,
	Item,
	LinkItem,
	Input,
	List,
	Separator,
	Shortcut,
	Loading,
	//
	Root as Command,
	Dialog as CommandDialog,
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
