<script lang="ts" module>
	export interface TimelineProps {
		class?: string;
		children?: any;
		/**
		 * Controlled open state for two-way binding
		 */
		open?: boolean;
		/**
		 * Default open state when uncontrolled
		 * @default true
		 */
		defaultOpen?: boolean;
	}
</script>

<script lang="ts">
	import * as Collapsible from '$lib/components/collapsible/index.js';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';

	let {
		class: className,
		children,
		open = $bindable(true),
		defaultOpen = true,
	}: TimelineProps = $props();

	// Initialize open state with defaultOpen if not explicitly set
	$effect(() => {
		if (open === undefined) {
			open = defaultOpen;
		}
	});
</script>

<Collapsible.Root bind:open class="w-full {className}">
	<Collapsible.Trigger class="w-full">
		<div class="flex flex-row w-full justify-end items-end">
			{#if open}
				<ChevronDown class="h-4 w-4" />
			{:else}
				<ChevronUp class="h-4 w-4" />
			{/if}
		</div>
	</Collapsible.Trigger>
	<Collapsible.Content>
		{@render children?.()}
	</Collapsible.Content>
</Collapsible.Root>
