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
		/**
		 * Label to display at the top of the timeline
		 * @default "Now"
		 */
		label?: string;
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
		label = 'Now',
	}: TimelineProps = $props();

	// Initialize open state with defaultOpen if not explicitly set
	$effect(() => {
		if (defaultOpen !== undefined) {
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
	{#if !open}
		<div class="flex flex-col">
			<div class="flex flex-row mb-4 h-8">
				<div class="flex w-1/2 items-center justify-end">
					<div class="flex flex-row w-max gap-2">
						{@render children?.()}
					</div>
				</div>
				<div class="flex w-fit justify-center pl-2 ml-4 border-l-2 items-center">
					{label}
				</div>
			</div>
		</div>
	{/if}

	<Collapsible.Content>
		<div class="flex flex-col">
			<div class="flex w-full justify-center mb-4">{label}</div>
			<div class="flex flex-row mb-4 h-[100px]">
				<div class="flex w-1/2 items-center justify-end">
					<div class="flex flex-row w-max gap-2">
						{@render children?.()}
					</div>
				</div>
				<div class="flex w-[200px] gap-2 h-full">
					<div class="h-full w-[5px] border rounded-full"></div>
				</div>
			</div>
		</div>
	</Collapsible.Content>
</Collapsible.Root>
