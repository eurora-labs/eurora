import Description from '$lib/components/alert/alert-description.svelte';
import Title from '$lib/components/alert/alert-title.svelte';
import Root from '$lib/components/alert/alert.svelte';
export { alertVariants, type AlertVariant } from './alert.svelte';

export {
	Root,
	Description,
	Title,
	//
	Root as Alert,
	Description as AlertDescription,
	Title as AlertTitle,
};
