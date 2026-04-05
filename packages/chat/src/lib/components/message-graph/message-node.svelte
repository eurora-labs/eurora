<script lang="ts">
	import * as FlowNode from '@eurora/ui/components/ai-elements/flow-node/index';
	import { Response as MessageResponse } from '@eurora/ui/components/ai-elements/message/index';

	export interface MessageNodeData {
		role: 'user' | 'assistant';
		content: string;
		label?: string;
		siblingLabel?: string;
		handles: { target: boolean; source: boolean };
		ondblclick?: () => void;
	}

	let { data }: { data: MessageNodeData } = $props();

	const roleLabel = $derived(data.label ?? (data.role === 'user' ? 'User' : 'Assistant'));
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="flex flex-col items-center gap-2" ondblclick={data.ondblclick}>
	<FlowNode.Root handles={data.handles}>
		<FlowNode.Header>
			<div class="flex items-center justify-between">
				<FlowNode.Title>{roleLabel}</FlowNode.Title>
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
