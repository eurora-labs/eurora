import Close from '$lib/components/sheet/sheet-close.svelte';
import Content from '$lib/components/sheet/sheet-content.svelte';
import Description from '$lib/components/sheet/sheet-description.svelte';
import Footer from '$lib/components/sheet/sheet-footer.svelte';
import Header from '$lib/components/sheet/sheet-header.svelte';
import Overlay from '$lib/components/sheet/sheet-overlay.svelte';
import Title from '$lib/components/sheet/sheet-title.svelte';
import Trigger from '$lib/components/sheet/sheet-trigger.svelte';
import { Dialog as SheetPrimitive } from 'bits-ui';

const Root = SheetPrimitive.Root;
const Portal = SheetPrimitive.Portal;

export {
	Root,
	Close,
	Trigger,
	Portal,
	Overlay,
	Content,
	Header,
	Footer,
	Title,
	Description,
	//
	Root as Sheet,
	Close as SheetClose,
	Trigger as SheetTrigger,
	Portal as SheetPortal,
	Overlay as SheetOverlay,
	Content as SheetContent,
	Header as SheetHeader,
	Footer as SheetFooter,
	Title as SheetTitle,
	Description as SheetDescription,
};
