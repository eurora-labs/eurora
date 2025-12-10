<script lang="ts" module>
	export interface TimelineProps {
		class?: string;
		children?: any;
		align?: 'left' | 'right';
		/**
		 * Whether the timeline can be collapsed
		 * @default false
		 */
		collapsible?: boolean;
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
		 * Position of the collapse trigger
		 * @default 'top'
		 */
		triggerPosition?: 'top' | 'bottom';
	}
</script>

<script lang="ts">
	import { ScrollArea } from '$lib/components/scroll-area/index.js';
	import * as Collapsible from '$lib/components/collapsible/index.js';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import ChevronUp from '@lucide/svelte/icons/chevron-up';

	let {
		class: className,
		children,
		align = 'left',
		collapsible = false,
		open = $bindable(true),
		defaultOpen = true,
		triggerPosition = 'top'
	}: TimelineProps = $props();

	let scrollAreaRef = $state<HTMLDivElement>();

	// Initialize open state with defaultOpen if not explicitly set
	$effect(() => {
		if (open === undefined) {
			open = defaultOpen;
		}
	});

	const justifyClass = $derived(align === 'right' ? 'justify-end' : 'justify-start');
</script>

{#if collapsible}
	<Collapsible.Root bind:open class="w-full {className}">
		{#if triggerPosition === 'top'}
			<Collapsible.Trigger class="w-full">
				<div class="flex flex-row w-full justify-end items-end">
					{#if open}
						<ChevronDown class="h-4 w-4" />
					{:else}
						<ChevronUp class="h-4 w-4" />
					{/if}
				</div>
			</Collapsible.Trigger>
		{/if}
		<Collapsible.Content>
			{@render children?.()}
		</Collapsible.Content>
		{#if triggerPosition === 'bottom'}
			<Collapsible.Trigger class="w-full">
				<div class="flex flex-row w-full justify-end items-end">
					{#if open}
						<ChevronDown class="h-4 w-4" />
					{:else}
						<ChevronUp class="h-4 w-4" />
					{/if}
				</div>
			</Collapsible.Trigger>
		{/if}
	</Collapsible.Root>
{:else}
	<ScrollArea
		ref={scrollAreaRef}
		orientation="horizontal"
		class="w-full whitespace-nowrap {className}"
	>
		<div class="flex w-full {justifyClass}">
			<div class="flex flex-row w-max gap-2">
				{@render children?.()}
			</div>
		</div>
	</ScrollArea>
{/if}
