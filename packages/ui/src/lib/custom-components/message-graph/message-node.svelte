<script lang="ts">
	import * as Attachment from '$lib/components/ai-elements/attachments/index';
	import * as FlowNode from '$lib/components/ai-elements/flow-node/index';
	import { Response as MessageResponse } from '$lib/components/ai-elements/message/index';

	export interface MessageNodeData {
		role: 'user' | 'assistant';
		content: string;
		label?: string;
		assets?: { id: string; name: string }[];
		siblingLabel?: string;
		handles: { target: boolean; source: boolean };
		ondblclick?: () => void;
	}

	let { data }: { data: MessageNodeData } = $props();

	const roleLabel = $derived(data.label ?? (data.role === 'user' ? 'User' : 'Assistant'));
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="flex flex-col items-center gap-2" ondblclick={data.ondblclick}>
	{#if data.assets?.length}
		<Attachment.Root variant="inline">
			{#each data.assets as asset (asset.id)}
				<Attachment.Item data={{ type: 'file', id: asset.id, filename: asset.name }}>
					<Attachment.Info />
				</Attachment.Item>
			{/each}
		</Attachment.Root>
	{/if}
	<FlowNode.Root handles={data.handles}>
		<FlowNode.Header>
			<div class="flex items-center justify-between">
				<FlowNode.Title>{roleLabel}</FlowNode.Title>
				{#if data.siblingLabel}
					<span class="text-muted-foreground text-xs">{data.siblingLabel}</span>
				{/if}
			</div>
		</FlowNode.Header>
		<FlowNode.Content>
			{#if data.role === 'assistant' && data.content.trim().length > 0}
				<div class="text-muted-foreground line-clamp-3 text-sm">
					<MessageResponse content={data.content} />
				</div>
			{:else}
				<p class="text-muted-foreground line-clamp-3 text-sm">{data.content}</p>
			{/if}
		</FlowNode.Content>
	</FlowNode.Root>
</div>
