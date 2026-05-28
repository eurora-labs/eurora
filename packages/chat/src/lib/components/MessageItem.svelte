<script lang="ts" module>
	import type { MessageNode } from '$lib/models/messages/index.js';
	import type { BranchDirection } from '$lib/services/thread/thread-service.js';

	interface MessageItemProps {
		node: MessageNode;
		isStreaming: boolean;
		isAnyStreaming: boolean;
		/** Streaming text content. Read only when `isStreaming` is true. */
		streamingContent: string;
		/** Streaming reasoning. Read only when `isStreaming` is true. */
		streamingReasoning: string;
		onCopy?: (content: string) => void;
		onEdit?: (messageId: string, newText: string) => void;
		onRegenerate?: (messageId: string) => void;
		onSwitchBranch: (messageId: string, direction: BranchDirection) => void;
	}
</script>

<script lang="ts">
	import { readAssetChips, readReasoningContent } from '$lib/models/messages/index.js';
	import { getTextContent, messageId } from '$lib/utils/message-content.js';
	import { middleTruncate } from '$lib/utils/text.js';
	import * as Attachment from '@eurora/ui/components/ai-elements/attachments/index';
	import * as Message from '@eurora/ui/components/ai-elements/message/index';
	import * as Reasoning from '@eurora/ui/components/ai-elements/reasoning/index';
	import { Shimmer } from '@eurora/ui/components/ai-elements/shimmer/index';
	import { Button } from '@eurora/ui/components/button/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
	import { useDebounce } from 'runed';
	import { tick } from 'svelte';

	const {
		node,
		isStreaming,
		isAnyStreaming,
		streamingContent,
		streamingReasoning,
		onCopy,
		onEdit,
		onRegenerate,
		onSwitchBranch,
	}: MessageItemProps = $props();

	// Conditional reads keep past messages decoupled from the streaming
	// signals: when `isStreaming` is false, the streamingContent / streamingReasoning
	// branches are never evaluated, so updates to those props do not invalidate
	// this component.
	const id = $derived(messageId(node));
	const user = $derived(node.message?.type === 'human');
	const content = $derived(isStreaming ? streamingContent : getTextContent(node));
	const reasoning = $derived(
		isStreaming ? streamingReasoning : readReasoningContent(node.message),
	);
	const assetChips = $derived(readAssetChips(node.message));
	const siblings = $derived(node.children ?? []);

	let copied = $state(false);
	const resetCopiedSoon = useDebounce(() => {
		copied = false;
	}, 2000);

	let editing = $state(false);
	let editText = $state('');
	let editTextarea = $state<HTMLTextAreaElement | null>(null);

	const showActions = $derived(user ? !isAnyStreaming : content.trim().length > 0);

	function handleCopy() {
		onCopy?.(content);
		copied = true;
		resetCopiedSoon();
	}

	async function startEdit() {
		editing = true;
		editText = content;
		await tick();
		editTextarea?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
		editTextarea?.focus();
	}

	function cancelEdit() {
		editing = false;
		editText = '';
	}

	function submitEdit() {
		if (!editing) return;
		const text = editText.trim();
		if (!text) return;
		const submittedId = id;
		editing = false;
		editText = '';
		onEdit?.(submittedId, text);
	}

	function handleEditKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			submitEdit();
		} else if (e.key === 'Escape') {
			cancelEdit();
		}
	}
</script>

{#snippet siblingNav()}
	{#if siblings.length > 1}
		<Message.Action
			tooltip="Previous"
			disabled={node.sibling_index === 0}
			onclick={() => onSwitchBranch(id, -1)}
		>
			<ChevronLeftIcon />
		</Message.Action>
		<span class="text-muted-foreground flex items-center text-xs">
			{node.sibling_index + 1} / {siblings.length}
		</span>
		<Message.Action
			tooltip="Next"
			disabled={node.sibling_index === siblings.length - 1}
			onclick={() => onSwitchBranch(id, 1)}
		>
			<ChevronRightIcon />
		</Message.Action>
	{/if}
{/snippet}

{#if content.length > 0 || assetChips.length > 0 || !user}
	<Message.Root from={user ? 'user' : 'assistant'} data-message-id={id}>
		{#if user && assetChips.length > 0}
			<Attachment.Root variant="inline" class="ml-auto">
				{#each assetChips as chip (chip.id)}
					<Attachment.Item
						data={{
							type: 'file',
							id: chip.id,
							filename: middleTruncate(chip.name),
						}}
					>
						<Attachment.Preview />
						<Attachment.Info />
					</Attachment.Item>
				{/each}
			</Attachment.Root>
		{/if}
		{#if reasoning}
			<Reasoning.Root {isStreaming}>
				<Reasoning.Trigger />
				<Reasoning.Content streaming={isStreaming} children={reasoning} />
			</Reasoning.Root>
		{/if}
		{#if user && editing}
			<div class="flex w-full flex-col gap-2">
				<textarea
					bind:this={editTextarea}
					class="bg-muted/50 border-border w-full resize-none rounded-lg border p-3 focus:outline-none"
					bind:value={editText}
					onkeydown={handleEditKeydown}
					rows={3}
				></textarea>
				<div class="flex justify-end gap-2">
					<Button variant="ghost" size="sm" onclick={cancelEdit}>Cancel</Button>
					<Button size="sm" onclick={submitEdit}>Send</Button>
				</div>
			</div>
		{:else}
			<Message.Content>
				{#if content.trim().length > 0}
					<Message.Response {content} streaming={isStreaming} />
				{:else if isStreaming && !reasoning}
					<Shimmer>Thinking</Shimmer>
				{/if}
			</Message.Content>
		{/if}
		{#if !isStreaming && !editing && showActions}
			<Message.Actions class={user ? 'self-end' : ''}>
				{@render siblingNav()}
				{#if onCopy}
					<Message.Action tooltip="Copy" onclick={handleCopy}>
						{#if copied}
							<CheckIcon />
						{:else}
							<CopyIcon />
						{/if}
					</Message.Action>
				{/if}
				{#if user && onEdit}
					<Message.Action tooltip="Edit" onclick={startEdit}>
						<PencilIcon />
					</Message.Action>
				{/if}
				{#if !user && onRegenerate}
					<Message.Action tooltip="Regenerate" onclick={() => onRegenerate(id)}>
						<RotateCcwIcon />
					</Message.Action>
				{/if}
			</Message.Actions>
		{/if}
	</Message.Root>
{/if}
