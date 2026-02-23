<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Switch } from '$lib/components/switch/index.js';
	import Eye from '@lucide/svelte/icons/eye';
	import EyeOff from '@lucide/svelte/icons/eye-off';
	import { useEnvironmentVariables } from './environment-variables-context.svelte.js';

	let {
		class: className,
		...restProps
	}: {
		class?: string;
	} & Record<string, unknown> = $props();

	const ctx = useEnvironmentVariables();
</script>

<div data-slot="environment-variables-toggle" class={cn('flex items-center gap-2', className)}>
	<span class="text-muted-foreground text-xs">
		{#if ctx.showValues}
			<Eye size={14} />
		{:else}
			<EyeOff size={14} />
		{/if}
	</span>
	<Switch
		aria-label="Toggle value visibility"
		checked={ctx.showValues}
		onCheckedChange={(checked) => ctx.setShowValues(checked)}
		{...restProps}
	/>
</div>
