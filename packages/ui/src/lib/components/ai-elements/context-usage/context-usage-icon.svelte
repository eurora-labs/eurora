<script lang="ts">
	import { getContextUsageContext } from './context-usage-context.svelte.js';

	const ICON_RADIUS = 10;
	const ICON_VIEWBOX = 24;
	const ICON_CENTER = 12;
	const ICON_STROKE_WIDTH = 2;

	let ctx = getContextUsageContext();

	let circumference = $derived(2 * Math.PI * ICON_RADIUS);
	let usedPercent = $derived(ctx.usedTokens / ctx.maxTokens);
	let dashOffset = $derived(circumference * (1 - usedPercent));
</script>

<svg
	data-slot="context-usage-icon"
	aria-label="Model context usage"
	height="20"
	role="img"
	style:color="currentcolor"
	viewBox="0 0 {ICON_VIEWBOX} {ICON_VIEWBOX}"
	width="20"
>
	<circle
		cx={ICON_CENTER}
		cy={ICON_CENTER}
		fill="none"
		opacity="0.25"
		r={ICON_RADIUS}
		stroke="currentColor"
		stroke-width={ICON_STROKE_WIDTH}
	/>
	<circle
		cx={ICON_CENTER}
		cy={ICON_CENTER}
		fill="none"
		opacity="0.7"
		r={ICON_RADIUS}
		stroke="currentColor"
		stroke-dasharray="{circumference} {circumference}"
		stroke-dashoffset={dashOffset}
		stroke-linecap="round"
		stroke-width={ICON_STROKE_WIDTH}
		style:transform="rotate(-90deg)"
		style:transform-origin="center"
	/>
</svg>
