<script lang="ts">
	import type { SearchResult } from '../types/search-result';
	import { SearchResultCategory } from '../types/search-result';
	import { File, Globe, Folder, Database, Layers, Package } from '@lucide/svelte';

	export let result: SearchResult;
	export let selected = false;
	export let index = 0;

	// Handle click on result item
	function handleClick() {
		if (result.action) {
			result.action();
		}
	}

	// Get icon based on result category
	function getIcon(category: SearchResultCategory) {
		switch (category) {
			case SearchResultCategory.APPLICATION:
				return Package;
			case SearchResultCategory.FILE:
				return File;
			case SearchResultCategory.WEBSITE:
				return Globe;
			case SearchResultCategory.FOLDER:
				return Folder;
			case SearchResultCategory.DRIVE:
				return Database;
			case SearchResultCategory.OTHER:
			default:
				return Layers;
		}
	}

	$: IconComponent = getIcon(result.category);
	$: customIcon = result.icon || '';
</script>

<div
	class="flex cursor-pointer items-center gap-3 rounded p-3 hover:bg-gray-100 {selected
		? 'bg-gray-100'
		: ''}"
	on:click={handleClick}
	role="option"
	aria-selected={selected}
	data-index={index}
>
	{#if customIcon}
		<img src={customIcon} alt="" class="h-6 w-6 flex-shrink-0" />
	{:else}
		<div class="h-6 w-6 flex-shrink-0 text-gray-500">
			<svelte:component this={IconComponent} size={24} />
		</div>
	{/if}

	<div class="min-w-0 flex-grow">
		<p class="truncate font-medium text-gray-900">{result.title}</p>
		{#if result.description}
			<p class="truncate text-sm text-gray-500">{result.description}</p>
		{/if}
	</div>

	<div class="rounded bg-gray-200 px-2 py-1 text-xs text-gray-700">
		{result.source}
	</div>
</div>
