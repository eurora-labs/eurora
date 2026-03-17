<script lang="ts">
	import * as FlowNode from '$lib/components/ai-elements/flow-node/index';
	import * as Attachment from '$lib/components/ai-elements/attachments/index';

	interface MessageNodeData {
		role: 'user' | 'assistant';
		content: string;
		assets?: { id: string; name: string }[];
		siblingLabel?: string;
		handles: { target: boolean; source: boolean };
	}

	let { data }: { data: MessageNodeData } = $props();

	const roleLabel = $derived(data.role === 'user' ? 'User' : 'Assistant');
</script>

<div class="flex flex-col items-center gap-2">
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
			<p class="text-muted-foreground line-clamp-3 text-sm">{data.content}</p>
		</FlowNode.Content>
	</FlowNode.Root>
</div>
