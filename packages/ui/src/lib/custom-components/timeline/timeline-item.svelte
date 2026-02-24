<script lang="ts" module>
	export interface TimelineItemProps {
		color?: string | null;
		iconBg?: string | null;
		highlighted?: boolean;
		iconSrc?: string | null;
		name?: string;
	}
</script>

<script lang="ts">
	let {
		color = 'white',
		iconBg = 'white',
		highlighted = false,
		iconSrc,
		name = '',
	}: TimelineItemProps = $props();

	let borderColor = $derived(color === 'white' ? 'black' : (color ?? 'black'));
	let resolvedIconBg = $derived(iconBg ?? 'white');
	let letterColor = $derived(resolvedIconBg === 'black' ? 'white' : 'black');
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
		{#if iconSrc}
			<img
				src={iconSrc}
				alt={name}
				class="w-8 h-8 rounded-full p-1"
				style="background-color: {resolvedIconBg};"
			/>
		{:else}
			<div
				class="w-8 h-8 rounded-full p-1 flex items-center justify-center"
				style="background-color: {resolvedIconBg}; color: {letterColor};"
			>
				{name.charAt(0).toUpperCase()}
			</div>
		{/if}
	</div>
</div>
