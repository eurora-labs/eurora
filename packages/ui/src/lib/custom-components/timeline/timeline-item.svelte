<script lang="ts" module>
	export interface TimelineItemProps {
		color?: string | null;
		highlighted?: boolean;
		iconSrc?: string | null;
		name?: string;
		/**
		 * Pixel length of the visible line extension beyond the icon, split
		 * equally above and below. The rendered line element itself is taller
		 * than this — it spans the full item, with the icon stacked on top of
		 * its midpoint — but only this many pixels show past the icon. Callers
		 * that want a duration-proportional line pass the computed value here;
		 * omit it for the default.
		 */
		connectorHeight?: number | null;
	}

	const DEFAULT_CONNECTOR_HEIGHT_PX = 8;
	const ICON_PX = 32;
</script>

<script lang="ts">
	let {
		color = null,
		highlighted = false,
		iconSrc = null,
		name = '',
		connectorHeight = null,
	}: TimelineItemProps = $props();

	let resolvedConnector = $derived(color ?? 'var(--sidebar-border)');
	let resolvedConnectorHeight = $derived(connectorHeight ?? DEFAULT_CONNECTOR_HEIGHT_PX);
	let lineHeight = $derived(resolvedConnectorHeight + ICON_PX);
</script>

<li
	class="grid place-items-center"
	style="filter: {highlighted ? 'saturate(2)' : 'saturate(0.6)'};"
	title={name}
>
	<span
		class="timeline-connector col-start-1 row-start-1 w-[1.4rem] rounded-full"
		style="background-color: {resolvedConnector}; height: {lineHeight}px;"
		aria-hidden="true"
	></span>
	{#if iconSrc}
		<img src={iconSrc} alt={name} class="col-start-1 row-start-1 size-8" />
	{:else}
		<div
			class="text-sidebar-foreground col-start-1 row-start-1 flex size-8 items-center justify-center text-sm font-medium"
		>
			{name.charAt(0).toUpperCase()}
		</div>
	{/if}
</li>
