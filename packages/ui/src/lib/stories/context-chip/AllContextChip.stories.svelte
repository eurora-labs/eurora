<script module lang="ts">
	import { ContextChip } from '$lib/custom-components/context-chip/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Inputs / ContextChip',
		args: {
			label: 'ContextChip'
		}
	});
	const defaultProps = {
		label: 'ContextChip',
		reversedDirection: false
	};
</script>

{#snippet contextChipGroup({
	variant,
	label,
	style,
	reversedDirection
}: {
	variant?: 'default' | 'primary' | 'secondary' | 'destructive' | 'outline';
	label: string;
	style?: 'default';
	reversedDirection?: boolean;
})}
	<div class="group"></div>
{/snippet}
