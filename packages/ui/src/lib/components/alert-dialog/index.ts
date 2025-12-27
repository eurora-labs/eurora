import Action from '$lib/components/alert-dialog/alert-dialog-action.svelte';
import Cancel from '$lib/components/alert-dialog/alert-dialog-cancel.svelte';
import Content from '$lib/components/alert-dialog/alert-dialog-content.svelte';
import Description from '$lib/components/alert-dialog/alert-dialog-description.svelte';
import Footer from '$lib/components/alert-dialog/alert-dialog-footer.svelte';
import Header from '$lib/components/alert-dialog/alert-dialog-header.svelte';
import Overlay from '$lib/components/alert-dialog/alert-dialog-overlay.svelte';
import Title from '$lib/components/alert-dialog/alert-dialog-title.svelte';
import Trigger from '$lib/components/alert-dialog/alert-dialog-trigger.svelte';
import { AlertDialog as AlertDialogPrimitive } from 'bits-ui';

const Root = AlertDialogPrimitive.Root;
const Portal = AlertDialogPrimitive.Portal;

export {
	Root,
	Title,
	Action,
	Cancel,
	Portal,
	Footer,
	Header,
	Trigger,
	Overlay,
	Content,
	Description,
	//
	Root as AlertDialog,
	Title as AlertDialogTitle,
	Action as AlertDialogAction,
	Cancel as AlertDialogCancel,
	Portal as AlertDialogPortal,
	Footer as AlertDialogFooter,
	Header as AlertDialogHeader,
	Trigger as AlertDialogTrigger,
	Overlay as AlertDialogOverlay,
	Content as AlertDialogContent,
	Description as AlertDialogDescription,
};
