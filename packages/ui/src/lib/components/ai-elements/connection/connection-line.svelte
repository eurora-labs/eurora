<script lang="ts">
	const HALF = 0.5;

	interface ConnectionLineProps {
		fromX: number;
		fromY: number;
		toX: number;
		toY: number;
	}

	let { fromX, fromY, toX, toY }: ConnectionLineProps = $props();

	let path = $derived(
		`M${fromX},${fromY} C ${fromX + (toX - fromX) * HALF},${fromY} ${fromX + (toX - fromX) * HALF},${toY} ${toX},${toY}`,
	);
</script>

<g data-slot="connection-line">
	<path class="animated" d={path} fill="none" stroke="var(--color-ring)" stroke-width={1} />
	<circle cx={toX} cy={toY} fill="#fff" r={3} stroke="var(--color-ring)" stroke-width={1} />
</g>
