import Close from '$lib/components/dialog/dialog-close.svelte';
import Content from '$lib/components/dialog/dialog-content.svelte';
import Description from '$lib/components/dialog/dialog-description.svelte';
import Footer from '$lib/components/dialog/dialog-footer.svelte';

import Header from '$lib/components/dialog/dialog-header.svelte';
import Overlay from '$lib/components/dialog/dialog-overlay.svelte';
import Title from '$lib/components/dialog/dialog-title.svelte';
import Trigger from '$lib/components/dialog/dialog-trigger.svelte';
import { Dialog as DialogPrimitive } from 'bits-ui';

const Root = DialogPrimitive.Root;
const Portal = DialogPrimitive.Portal;

export {
	Root,
	Title,
	Portal,
	Footer,
	Header,
	Trigger,
	Overlay,
	Content,
	Description,
	Close,
	//
	Root as Dialog,
	Title as DialogTitle,
	Portal as DialogPortal,
	Footer as DialogFooter,
	Header as DialogHeader,
	Trigger as DialogTrigger,
	Overlay as DialogOverlay,
	Content as DialogContent,
	Description as DialogDescription,
	Close as DialogClose,
};
