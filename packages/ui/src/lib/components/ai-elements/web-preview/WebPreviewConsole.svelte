<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import * as Collapsible from '$lib/components/collapsible/index.js';
	import ChevronDownIcon from '@lucide/svelte/icons/chevron-down';
	import { getWebPreviewContext } from './web-preview-context.svelte.js';

	export interface ConsoleLog {
		level: 'log' | 'warn' | 'error';
		message: string;
		timestamp: Date;
	}

	interface Props extends HTMLAttributes<HTMLDivElement> {
		logs?: ConsoleLog[];
		children?: Snippet;
	}

	let { class: className, logs = [], children, ...restProps }: Props = $props();

	let context = getWebPreviewContext();
</script>

<Collapsible.Root
	data-slot="web-preview-console"
	class={cn('border-t bg-muted/50 font-mono text-sm', className)}
	bind:open={context.consoleOpen}
	{...restProps}
>
	<Collapsible.Trigger>
		{#snippet child({ props })}
			<Button
				{...props}
				class="flex w-full items-center justify-between p-4 text-left font-medium hover:bg-muted/50"
				variant="ghost"
			>
				Console
				<ChevronDownIcon
					class={cn('h-4 w-4 transition-transform duration-200', context.consoleOpen && 'rotate-180')}
				/>
			</Button>
		{/snippet}
	</Collapsible.Trigger>
	<Collapsible.Content class="px-4 pb-4">
		<div class="max-h-48 space-y-1 overflow-y-auto">
			{#if logs.length === 0}
				<p class="text-muted-foreground">No console output</p>
			{:else}
				{#each logs as log, index (`${log.timestamp.getTime()}-${index}`)}
					<div
						class={cn(
							'text-xs',
							log.level === 'error' && 'text-destructive',
							log.level === 'warn' && 'text-yellow-600',
							log.level === 'log' && 'text-foreground',
						)}
					>
						<span class="text-muted-foreground">
							{log.timestamp.toLocaleTimeString()}
						</span>
						{' '}
						{log.message}
					</div>
				{/each}
			{/if}
			{@render children?.()}
		</div>
	</Collapsible.Content>
</Collapsible.Root>
