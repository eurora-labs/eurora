<script lang="ts">
	import { type Variant, contextChipVariants } from './index.js';
	import { cn } from '@eurora/ui/utils.js';

	let className: string | undefined | null = undefined;
	export let href: string | undefined = undefined;
	export let variant: Variant = 'default';
	export let onClick: ((event: MouseEvent) => void) | undefined = undefined;
	export { className as class };
</script>

<svelte:element
	this={href ? 'a' : 'span'}
	{href}
	role={onClick ? 'button' : undefined}
	on:click={onClick}
	class={cn(contextChipVariants({ variant, className }))}
	{...$$restProps}
>
	<slot />
</svelte:element>
