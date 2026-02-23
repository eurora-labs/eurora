<script lang="ts">
	import type { Snippet } from 'svelte';
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

	interface Props {
		class?: string;
		logs?: ConsoleLog[];
		children?: Snippet;
	}

	let { class: className, logs = [], children }: Props = $props();

	let context = getWebPreviewContext();
</script>

<Collapsible.Root
	class={cn('border-t bg-muted/50 font-mono text-sm', className)}
	bind:open={context.consoleOpen}
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
					class={cn(
						'h-4 w-4 transition-transform duration-200',
						context.consoleOpen && 'rotate-180',
					)}
				/>
			</Button>
		{/snippet}
	</Collapsible.Trigger>
	<Collapsible.Content
		class={cn(
			'px-4 pb-4',
			'data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2 outline-none data-[state=closed]:animate-out data-[state=open]:animate-in',
		)}
	>
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
