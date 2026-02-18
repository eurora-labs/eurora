<script lang="ts" module>
	export interface TimelineProps {
		class?: string;
		children?: any;
		open?: boolean;
		defaultOpen?: boolean;
	}
</script>

<script lang="ts">
	let {
		class: className,
		children,
		open = $bindable(true),
		defaultOpen = true,
	}: TimelineProps = $props();

	$effect(() => {
		if (defaultOpen !== undefined) {
			open = defaultOpen;
		}
	});
</script>

<div class="flex flex-col {className} ">
	<div class="relative mb-4 h-8 overflow-hidden group-data-[collapsible=icon]:overflow-visible">
		<div
			class="absolute right-[calc(50%-3.125rem)] flex flex-row gap-2 items-center group-data-[collapsible=icon]:static group-data-[collapsible=icon]:justify-center"
		>
			{@render children?.()}
		</div>
	</div>
</div>
