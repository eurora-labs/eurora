<script lang="ts">
	interface Props {
		tokens?: number;
		costText?: string;
	}

	let { tokens, costText }: Props = $props();

	let formattedTokens = $derived(
		tokens === undefined
			? '\u2014'
			: new Intl.NumberFormat('en-US', { notation: 'compact' }).format(tokens),
	);
</script>

<span data-slot="tokens-with-cost">
	{formattedTokens}
	{#if costText}
		<span class="text-muted-foreground ml-2">&bull; {costText}</span>
	{/if}
</span>
