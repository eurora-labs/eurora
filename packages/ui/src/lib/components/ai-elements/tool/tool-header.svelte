<script lang="ts">
	import type { Snippet } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { Badge } from '$lib/components/badge/index.js';
	import { CollapsibleTrigger } from '$lib/components/collapsible/index.js';
	import Wrench from '@lucide/svelte/icons/wrench';
	import ChevronDown from '@lucide/svelte/icons/chevron-down';
	import CheckCircle from '@lucide/svelte/icons/check-circle';
	import Circle from '@lucide/svelte/icons/circle';
	import Clock from '@lucide/svelte/icons/clock';
	import XCircle from '@lucide/svelte/icons/x-circle';

	type ToolPartState =
		| 'approval-requested'
		| 'approval-responded'
		| 'input-available'
		| 'input-streaming'
		| 'output-available'
		| 'output-denied'
		| 'output-error';

	let {
		class: className,
		title,
		type,
		state,
		toolName,
		children,
		...restProps
	}: {
		class?: string;
		title?: string;
		type: string;
		state: ToolPartState;
		toolName?: string;
		children?: Snippet;
	} = $props();

	const statusLabels: Record<ToolPartState, string> = {
		'approval-requested': 'Awaiting Approval',
		'approval-responded': 'Responded',
		'input-available': 'Running',
		'input-streaming': 'Pending',
		'output-available': 'Completed',
		'output-denied': 'Denied',
		'output-error': 'Error',
	};

	const statusIconColors: Record<ToolPartState, string> = {
		'approval-requested': 'text-yellow-600',
		'approval-responded': 'text-blue-600',
		'input-available': 'animate-pulse',
		'input-streaming': '',
		'output-available': 'text-green-600',
		'output-denied': 'text-orange-600',
		'output-error': 'text-red-600',
	};

	let derivedName = $derived(
		type === 'dynamic-tool' ? (toolName ?? '') : type.split('-').slice(1).join('-'),
	);
</script>

<CollapsibleTrigger
	data-slot="tool-header"
	class={cn('flex w-full items-center justify-between gap-4 p-3', className)}
	{...restProps}
>
	<div class="flex items-center gap-2">
		<Wrench class="size-4 text-muted-foreground" />
		<span class="font-medium">{title ?? derivedName}</span>
		<Badge class="gap-1.5 rounded-full text-xs" variant="secondary">
			{#if state === 'approval-requested'}
				<Clock class={cn('size-4', statusIconColors[state])} />
			{:else if state === 'approval-responded'}
				<CheckCircle class={cn('size-4', statusIconColors[state])} />
			{:else if state === 'input-available'}
				<Clock class={cn('size-4', statusIconColors[state])} />
			{:else if state === 'input-streaming'}
				<Circle class={cn('size-4', statusIconColors[state])} />
			{:else if state === 'output-available'}
				<CheckCircle class={cn('size-4', statusIconColors[state])} />
			{:else if state === 'output-denied'}
				<XCircle class={cn('size-4', statusIconColors[state])} />
			{:else if state === 'output-error'}
				<XCircle class={cn('size-4', statusIconColors[state])} />
			{/if}
			{statusLabels[state]}
		</Badge>
	</div>
	<ChevronDown
		class="size-4 text-muted-foreground transition-transform group-data-[state=open]:rotate-180"
	/>
</CollapsibleTrigger>
