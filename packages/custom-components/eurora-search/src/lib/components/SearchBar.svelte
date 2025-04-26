<script lang="ts">
	import { createEventDispatcher } from 'svelte';
	import { Search } from '@lucide/svelte';

	export let value = '';
	export let placeholder = 'Search for anything...';
	export let autofocus = true;

	const dispatch = createEventDispatcher<{
		search: string;
		input: string;
		keydown: KeyboardEvent;
	}>();

	let input: HTMLInputElement;

	function handleInput(e: Event) {
		const target = e.target as HTMLInputElement;
		value = target.value;
		dispatch('input', value);

		// If user is typing, perform search with slight debounce
		clearTimeout(searchDebounce);
		searchDebounce = setTimeout(() => {
			dispatch('search', value);
		}, 200);
	}

	function handleKeyDown(e: KeyboardEvent) {
		dispatch('keydown', e);

		// Immediately search on Enter key
		if (e.key === 'Enter') {
			clearTimeout(searchDebounce);
			dispatch('search', value);
		}
	}

	// Debounce search to avoid excessive searches while typing
	let searchDebounce: ReturnType<typeof setTimeout>;

	// Auto-focus the input when the component is mounted
	function handleFocus() {
		if (autofocus && input) {
			input.focus();
		}
	}

	$: if (input && autofocus) {
		handleFocus();
	}
</script>

<div class="relative w-full">
	<input
		bind:this={input}
		bind:value
		{placeholder}
		on:input={handleInput}
		on:keydown={handleKeyDown}
		class="h-12 w-full rounded-md border border-gray-300 pl-10 pr-4 text-base focus:border-blue-500 focus:outline-none focus:ring-2 focus:ring-blue-500"
	/>
</div>
