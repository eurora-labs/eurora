import NestedRoot from './drawer-nested.svelte';
import Overlay from './drawer-overlay.svelte';
import Title from './drawer-title.svelte';
import Close from '$lib/components/drawer/drawer-close.svelte';
import Content from '$lib/components/drawer/drawer-content.svelte';
import Description from '$lib/components/drawer/drawer-description.svelte';
import Footer from '$lib/components/drawer/drawer-footer.svelte';
import Header from '$lib/components/drawer/drawer-header.svelte';
import Trigger from '$lib/components/drawer/drawer-trigger.svelte';
import Root from '$lib/components/drawer/drawer.svelte';
import { Drawer as DrawerPrimitive } from 'vaul-svelte';

const Portal: typeof DrawerPrimitive.Portal = DrawerPrimitive.Portal;

export {
	Root,
	NestedRoot,
	Content,
	Description,
	Overlay,
	Footer,
	Header,
	Title,
	Trigger,
	Portal,
	Close,

	//
	Root as Drawer,
	NestedRoot as DrawerNestedRoot,
	Content as DrawerContent,
	Description as DrawerDescription,
	Overlay as DrawerOverlay,
	Footer as DrawerFooter,
	Header as DrawerHeader,
	Title as DrawerTitle,
	Trigger as DrawerTrigger,
	Portal as DrawerPortal,
	Close as DrawerClose,
};
