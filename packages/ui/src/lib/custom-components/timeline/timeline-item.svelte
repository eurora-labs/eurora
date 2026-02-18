<script lang="ts" module>
	export interface TimelineItemProps {
		color?: string | null;
		highlighted?: boolean;
		children?: any;
	}
</script>

<script lang="ts">
	let { color = 'white', highlighted = false, children }: TimelineItemProps = $props();

	let borderColor = $derived(color === 'white' ? 'black' : (color ?? 'black'));
</script>

<div
	class="relative min-w-25 group-data-[collapsible=icon]:min-w-0 shrink group-data-[collapsible=icon]:shrink-0 flex items-center justify-center rounded-2xl"
	style="filter: {highlighted ? 'saturate(2)' : 'saturate(0)'};"
>
	<div
		class="absolute w-full h-2 rounded-2xl top-1/2 -translate-y-1/2 border-solid border group-data-[collapsible=icon]:hidden"
		style="background-color: {color}; border-color: {borderColor};"
	></div>
	<div class="relative z-10 flex items-center justify-center w-fit text-sm text-center p-0 m-0">
		{@render children?.()}
	</div>
</div>
