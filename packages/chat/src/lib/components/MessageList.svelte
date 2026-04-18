<script lang="ts" module>
	import type { Snippet } from 'svelte';

	interface Props {
		emptyState?: Snippet;
		onCopy?: (content: string) => void;
		onEdit?: (messageId: string, newText: string) => void;
	}
</script>

<script lang="ts">
	import { CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { getTextContent } from '$lib/utils/message-content.js';
	import { middleTruncate } from '$lib/utils/text.js';
	import { inject } from '@eurora/shared/context';
	import * as Attachment from '@eurora/ui/components/ai-elements/attachments/index';
	import * as Conversation from '@eurora/ui/components/ai-elements/conversation/index';
	import { initStickToBottomContext } from '@eurora/ui/components/ai-elements/conversation/index';
	import * as Message from '@eurora/ui/components/ai-elements/message/index';
	import * as Reasoning from '@eurora/ui/components/ai-elements/reasoning/index';
	import { Shimmer } from '@eurora/ui/components/ai-elements/shimmer/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import { Skeleton } from '@eurora/ui/components/skeleton/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import { tick } from 'svelte';
	import type { ContentBlock } from '$lib/models/content-blocks/index.js';
	import type { AssetChip, MessageNode } from '$lib/models/messages/index.js';

	let { emptyState, onCopy, onEdit }: Props = $props();

	let copiedId = $state<string | null>(null);
	let editingId = $state<string | null>(null);
	let editText = $state('');
	let editTextarea = $state<HTMLTextAreaElement | null>(null);
	const chatService = inject(CHAT_SERVICE);
	const scrollContext = initStickToBottomContext();

	let prevThreadId: string | undefined;
	let prevStreamingId: string | null | undefined;

	$effect(() => {
		const threadId = chatService.activeThreadId;
		const streamingId = chatService.activeThread?.streamingMessageId ?? null;

		const threadChanged = threadId !== prevThreadId;
		const streamingStarted = streamingId !== null && streamingId !== prevStreamingId;

		if (threadChanged || streamingStarted) {
			scrollContext.reengageAutoScroll();
		}

		prevThreadId = threadId;
		prevStreamingId = streamingId;
	});

	function getContentBlocks(node: MessageNode): ContentBlock[] {
		const msg = node.message;
		if (!msg || msg.type === 'remove') return [];
		return msg.content;
	}

	function getReasoningContent(node: MessageNode): string {
		return getContentBlocks(node)
			.map((b) => (b.type === 'reasoning' ? (b.reasoning ?? '') : ''))
			.join('');
	}

	function isUser(node: MessageNode): boolean {
		return node.message?.type === 'human';
	}

	function getMessageId(node: MessageNode): string {
		return node.message.id;
	}

	function getAssetChips(node: MessageNode): AssetChip[] {
		return node.message.type === 'human' ? node.message.assetChips : [];
	}

	function handleCopy(content: string, messageId: string) {
		onCopy?.(content);
		copiedId = messageId;
		setTimeout(() => {
			if (copiedId === messageId) copiedId = null;
		}, 2000);
	}

	async function startEdit(messageId: string, content: string) {
		editingId = messageId;
		editText = content;
		await tick();
		editTextarea?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
		editTextarea?.focus();
	}

	function cancelEdit() {
		editingId = null;
		editText = '';
	}

	function submitEdit() {
		if (editingId === null) return;
		const text = editText.trim();
		if (!text) return;
		const id = editingId;
		editingId = null;
		editText = '';
		onEdit?.(id, text);
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

{#snippet siblingNav(node: MessageNode)}
	{#if node.children.length > 1}
		{@const activeId = getMessageId(node)}
		<Message.Action
			tooltip="Previous"
			disabled={node.siblingIndex === 0}
			onclick={() => {
				if (activeId && chatService.activeThreadId)
					chatService.switchBranch(chatService.activeThreadId, activeId, -1);
			}}
		>
			<ChevronLeftIcon />
		</Message.Action>
		<span class="text-muted-foreground flex items-center text-xs">
			{node.siblingIndex + 1} / {node.children.length}
		</span>
		<Message.Action
			tooltip="Next"
			disabled={node.siblingIndex === node.children.length - 1}
			onclick={() => {
				if (activeId && chatService.activeThreadId)
					chatService.switchBranch(chatService.activeThreadId, activeId, 1);
			}}
		>
			<ChevronRightIcon />
		</Message.Action>
	{/if}
{/snippet}

<Conversation.Root class="min-h-0 flex-1">
	<Conversation.Content>
		{#if chatService.activeThread?.messages.length === 0 && chatService.activeThread?.loading}
			<div class="loading-skeletons">
				{#each Array(4) as _, i}
					<div
						class="flex w-full max-w-[95%] flex-col gap-2 {i % 2 === 0
							? 'ml-auto items-end'
							: 'items-start'}"
					>
						<div class="flex flex-col gap-2 rounded-lg px-4 py-3">
							<Skeleton class="shimmer bg-muted h-4 w-48" />
							<Skeleton class="shimmer bg-muted h-4 w-36" />
							{#if i % 2 === 1}
								<Skeleton class="shimmer bg-muted h-4 w-56" />
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{:else if !chatService.activeThread?.messages.length}
			{#if emptyState}
				{@render emptyState()}
			{:else}
				<Empty.Root>
					<Empty.Header>
						<Empty.Title>No messages yet</Empty.Title>
					</Empty.Header>
				</Empty.Root>
			{/if}
		{/if}
		{#each chatService.activeThread?.messages as node}
			{@const content = getTextContent(node)}
			{@const user = isUser(node)}
			{@const reasoning = getReasoningContent(node)}
			{@const messageId = getMessageId(node)}
			{@const assetChips = getAssetChips(node)}
			{@const isStreaming = chatService.activeThread?.streamingMessageId === messageId}
			{#if content.length > 0 || assetChips.length > 0 || !user}
				<Message.Root from={user ? 'user' : 'assistant'} data-message-id={messageId}>
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
							<Reasoning.Content children={reasoning} />
						</Reasoning.Root>
					{/if}
					{#if user && editingId === messageId}
						<div class="flex w-full flex-col gap-2">
							<textarea
								bind:this={editTextarea}
								class="bg-muted/50 border-border w-full resize-none rounded-lg border p-3 focus:outline-none"
								bind:value={editText}
								onkeydown={handleEditKeydown}
								rows={3}
							></textarea>
							<div class="flex justify-end gap-2">
								<Button variant="ghost" size="sm" onclick={cancelEdit}>
									Cancel
								</Button>
								<Button size="sm" onclick={submitEdit}>Send</Button>
							</div>
						</div>
					{:else}
						<Message.Content>
							{#if content.trim().length > 0}
								<Message.Response {content} />
							{:else if isStreaming && !reasoning}
								<Shimmer>Thinking</Shimmer>
							{/if}
						</Message.Content>
					{/if}
					{#if !isStreaming && editingId !== messageId}
						{@const showActions = user
							? !chatService.activeThread?.streamingMessageId
							: content.trim().length > 0}
						{#if showActions}
							<Message.Actions class={user ? 'self-end' : ''}>
								{@render siblingNav(node)}
								{#if onCopy}
									<Message.Action
										tooltip="Copy"
										onclick={() => handleCopy(content, messageId)}
									>
										{#if copiedId === messageId}
											<CheckIcon />
										{:else}
											<CopyIcon />
										{/if}
									</Message.Action>
								{/if}
								{#if user && onEdit}
									<Message.Action
										tooltip="Edit"
										onclick={() => startEdit(messageId, content)}
									>
										<PencilIcon />
									</Message.Action>
								{/if}
							</Message.Actions>
						{/if}
					{/if}
				</Message.Root>
			{/if}
		{/each}
	</Conversation.Content>
</Conversation.Root>

<style>
	.loading-skeletons :global(.shimmer) {
		background-image: linear-gradient(
			110deg,
			transparent 25%,
			var(--muted-foreground) 37%,
			transparent 63%
		);
	}
</style>
