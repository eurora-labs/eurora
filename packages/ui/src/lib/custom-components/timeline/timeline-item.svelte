<script lang="ts" module>
	export interface TimelineItemProps {
		id?: string;
		color?: string | null;
		/**
		 * The currently-selected item in the rail. Renders with full
		 * saturation, a slight scale-up, and a ring in the accent colour;
		 * non-active items dim and shrink so the active one reads at a
		 * glance.
		 */
		active?: boolean;
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
		id,
		color = null,
		active = false,
		iconSrc = null,
		name = '',
		connectorHeight = null,
	}: TimelineItemProps = $props();

	let resolvedConnector = $derived(color ?? 'var(--sidebar-border)');
	let resolvedConnectorHeight = $derived(connectorHeight ?? DEFAULT_CONNECTOR_HEIGHT_PX);
	let lineHeight = $derived(resolvedConnectorHeight + ICON_PX);
</script>

<li
	{id}
	class="bg-sidebar"
	title={name}
	role="option"
	aria-selected={active}
	aria-current={active ? 'true' : undefined}
>
	<div
		class="grid place-items-center transition-[transform,filter] duration-150 ease-out"
		style="transform: scale({active ? 1.1 : 0.9}); filter: saturate({active ? 1.6 : 0.5});"
	>
		<span
			class="timeline-connector col-start-1 row-start-1 w-4 rounded-full"
			style="background-color: {resolvedConnector}; height: {lineHeight}px;"
			aria-hidden="true"
		></span>
		<span
			class="col-start-1 row-start-1 size-8 rounded-md shadow-md"
			class:ring-2={active}
			class:ring-offset-1={active}
			class:ring-offset-sidebar={active}
			style="background-color: {resolvedConnector}; --tw-ring-color: {resolvedConnector};"
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
	</div>
</li>
