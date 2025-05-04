<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Search } from '@lucide/svelte';
	import { cn } from '@eurora/ui/utils.js';

	let {
		ref = $bindable(null),
		class: className = '',
		value = $bindable(''),
		height = $bindable('100px'),
		placeholder = 'Type a command or search...',
		disabled = false,
		...restProps
	} = $props();

	const dispatch = createEventDispatcher();

	// Handle input event to update value and dispatch event
	function handleInput(event: Event) {
		const target = event.target as HTMLInputElement;
		value = target.value;
		dispatch('input', { value });
	}

	// Handle keydown for special keys like arrow keys, enter, escape
	function handleKeyDown(event: KeyboardEvent) {
		dispatch('keydown', event);
	}
</script>

<div class="flex items-center border-none px-3" data-command-input-wrapper="">
	<Search class="mr-2 shrink-0 opacity-50" size="40" style="color: rgba(0, 0, 0, 0.8);" />
	<input
		type="text"
		class={cn(
			'placeholder:text-[rgba(0, 0, 0, 0.25)] custom-input flex w-full rounded-md border-none bg-transparent py-3 text-base shadow-none outline-none focus:border-transparent focus:ring-0 disabled:cursor-not-allowed disabled:opacity-50 md:text-sm',
			className
		)}
		style="height: {height}; box-shadow: none; font-size: 40px; color: rgba(0, 0, 0, 0.5); padding-left: 0px; font-weight: 400;"
		bind:this={ref}
		bind:value
		{placeholder}
		{disabled}
		on:input={handleInput}
		on:keydown={handleKeyDown}
		role="combobox"
		aria-autocomplete="list"
		autocomplete="off"
		{...restProps}
	/>
</div>

<style>
	:global(.custom-input::placeholder) {
		color: rgba(0, 0, 0, 0.25);
	}
</style>
