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

	export { TimelineProps };
</script>

<div class="flex flex-col {className}">
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
