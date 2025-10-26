<script lang="ts" module>
	export interface TimelineItemProps {
		name: string;
		icon?: string | null;

		color?: string | null;
	}
</script>

<script lang="ts">
	let { color = 'white', icon, name }: TimelineItemProps = $props();

	let borderColor = $state('white');
	$effect(() => {
		console.log(color);
		if (color == 'white') {
			borderColor = 'black';
		} else {
			borderColor = color ?? 'black';
		}
	});
</script>

<div class="relative w-full h-[50px] flex items-center justify-center rounded-2xl">
	<div
		class="absolute w-full h-[15px] rounded-2xl top-1/2 -translate-y-1/2 border-solid border-1"
		style="background-color: {color}; border-color: {borderColor};"
	></div>
	<div class="relative z-10 flex items-center justify-center w-fit text-sm text-center p-0 m-0">
		{#if icon}<img src={icon} alt={name} class="w-4 h-4" />{:else}{name}{/if}
	</div>
</div>
