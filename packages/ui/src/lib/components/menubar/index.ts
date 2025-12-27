import CheckboxItem from '$lib/components/menubar/menubar-checkbox-item.svelte';
import Content from '$lib/components/menubar/menubar-content.svelte';
import GroupHeading from '$lib/components/menubar/menubar-group-heading.svelte';
import Group from '$lib/components/menubar/menubar-group.svelte';
import Item from '$lib/components/menubar/menubar-item.svelte';
import Label from '$lib/components/menubar/menubar-label.svelte';
import RadioItem from '$lib/components/menubar/menubar-radio-item.svelte';
import Separator from '$lib/components/menubar/menubar-separator.svelte';
import Shortcut from '$lib/components/menubar/menubar-shortcut.svelte';
import SubContent from '$lib/components/menubar/menubar-sub-content.svelte';
import SubTrigger from '$lib/components/menubar/menubar-sub-trigger.svelte';
import Trigger from '$lib/components/menubar/menubar-trigger.svelte';
import Root from '$lib/components/menubar/menubar.svelte';
import { Menubar as MenubarPrimitive } from 'bits-ui';

const Menu = MenubarPrimitive.Menu;
const Sub = MenubarPrimitive.Sub;
const RadioGroup = MenubarPrimitive.RadioGroup;

export {
	Root,
	CheckboxItem,
	Content,
	Item,
	RadioItem,
	Separator,
	Shortcut,
	SubContent,
	SubTrigger,
	Trigger,
	Menu,
	Group,
	Sub,
	RadioGroup,
	Label,
	GroupHeading,
	//
	Root as Menubar,
	CheckboxItem as MenubarCheckboxItem,
	Content as MenubarContent,
	Item as MenubarItem,
	RadioItem as MenubarRadioItem,
	Separator as MenubarSeparator,
	Shortcut as MenubarShortcut,
	SubContent as MenubarSubContent,
	SubTrigger as MenubarSubTrigger,
	Trigger as MenubarTrigger,
	Menu as MenubarMenu,
	Group as MenubarGroup,
	Sub as MenubarSub,
	RadioGroup as MenubarRadioGroup,
	Label as MenubarLabel,
	GroupHeading as MenubarGroupHeading,
};
