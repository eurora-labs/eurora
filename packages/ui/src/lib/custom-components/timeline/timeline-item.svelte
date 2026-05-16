<script lang="ts" module>
	export interface TimelineItemProps {
		color?: string | null;
		iconBg?: string | null;
		iconColor?: string | null;
		highlighted?: boolean;
		iconSrc?: string | null;
		name?: string;
	}
</script>

<script lang="ts">
	let {
		color = null,
		iconBg = null,
		iconColor = null,
		highlighted = false,
		iconSrc = null,
		name = '',
	}: TimelineItemProps = $props();

	let resolvedConnector = $derived(color ?? 'var(--sidebar-border)');
	let resolvedIconBg = $derived(iconBg ?? 'white');
	let resolvedIconColor = $derived(iconColor ?? 'black');
</script>

<li
	class="flex flex-col items-center"
	style="filter: {highlighted ? 'saturate(2)' : 'saturate(0.6)'};"
	title={name}
>
	<span
		class="timeline-connector h-2 w-0.5 rounded-full"
		style="background-color: {resolvedConnector};"
		aria-hidden="true"
	></span>
	{#if iconSrc}
		<img
			src={iconSrc}
			alt={name}
			class="size-8 rounded-full p-1"
			style="background-color: {resolvedIconBg};"
		/>
	{:else}
		<div
			class="flex size-8 items-center justify-center rounded-full text-sm font-medium"
			style="background-color: {resolvedIconBg}; color: {resolvedIconColor};"
		>
			{name.charAt(0).toUpperCase()}
		</div>
	{/if}
</li>
