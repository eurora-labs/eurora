<script lang="ts">
	import { Command as CommandPrimitive } from 'bits-ui';
	import { Search } from '@lucide/svelte';
	import { cn } from '@eurora/ui/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(''),
		height = $bindable('100px'),
		...restProps
	}: CommandPrimitive.InputProps = $props();
</script>

<div class=" flex items-center border-none px-3" data-command-input-wrapper="">
	<Search class="mr-2 shrink-0 opacity-50" size="40" style="color: rgba(0, 0, 0, 0.8);" />
	<CommandPrimitive.Input
		class={cn(
			'placeholder:text-muted-foreground custom-input flex w-full rounded-md border-none bg-transparent py-3 text-base shadow-none outline-none focus:border-transparent focus:ring-0 disabled:cursor-not-allowed disabled:opacity-50 md:text-sm',
			className
		)}
		style="height: {height}; box-shadow: none; font-size: 40px; color: rgba(0, 0, 0, 0.5); padding-left: 0px; font-weight: 400;"
		bind:ref
		bind:value
		{...restProps}
	/>
</div>

<style>
	:global(.custom-input::placeholder) {
		color: rgba(0, 0, 0, 0.25);
	}
</style>
