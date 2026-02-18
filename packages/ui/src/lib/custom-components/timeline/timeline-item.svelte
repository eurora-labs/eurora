<script lang="ts" module>
	export interface TimelineItemProps {
		color?: string | null;
		children?: any;
	}
</script>

<script lang="ts">
	let { color = 'white', children }: TimelineItemProps = $props();

	let borderColor = $state('white');
	$effect(() => {
		if (color === 'white') {
			borderColor = 'black';
		} else {
			borderColor = color ?? 'black';
		}
	});
</script>

<div
	class="relative min-w-25 group-data-[collapsible=icon]:min-w-0 shrink group-data-[collapsible=icon]:shrink-0 flex items-center justify-center rounded-2xl"
>
	<div
		class="absolute w-full h-2 rounded-2xl top-1/2 -translate-y-1/2 border-solid border group-data-[collapsible=icon]:hidden"
		style="background-color: {color}; border-color: {borderColor};"
	></div>
	<div class="relative z-10 flex items-center justify-center w-fit text-sm text-center p-0 m-0">
		{@render children?.()}
	</div>
</div>
