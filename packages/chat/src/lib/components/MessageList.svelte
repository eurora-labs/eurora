<script lang="ts" module>
	interface Props {
		emptyState?: Snippet;
		onCopy?: (content: string) => void;
		onEdit?: (messageId: string, newText: string) => void;
	}
</script>

<script lang="ts">
	import { CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Conversation from '@eurora/ui/components/ai-elements/conversation/index';
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
	import type { BaseMessageWithSibling } from '@eurora/shared/proto/agent_chain_pb.js';
	import type { Snippet } from 'svelte';

	let { emptyState, onCopy, onEdit }: Props = $props();

	let copiedId = $state<string | null>(null);
	let editingId = $state<string | null>(null);
	let editText = $state('');
	let editTextarea = $state<HTMLTextAreaElement | null>(null);
	const chatService = inject(CHAT_SERVICE);

	type OneofResult = { key: string; value: Record<string, unknown> } | null;

	function unwrapOneof(obj: unknown): OneofResult {
		if (!obj || typeof obj !== 'object') return null;
		const record = obj as Record<string, unknown>;
		if ('case' in record && record.case !== undefined) {
			return { key: record.case as string, value: record.value as Record<string, unknown> };
		}
		for (const k of Object.keys(record)) {
			if (k.startsWith('$') || k.startsWith('_')) continue;
			return { key: k.toLowerCase(), value: record[k] as Record<string, unknown> };
		}
		return null;
	}

	function getContentBlocks(node: BaseMessageWithSibling): Record<string, unknown>[] {
		const inner = unwrapOneof((node.message as Record<string, unknown>)?.message);
		if (!inner || inner.key === 'remove') return [];
		return (inner.value?.content as Record<string, unknown>[]) ?? [];
	}

	function getTextContent(node: BaseMessageWithSibling): string {
		return getContentBlocks(node)
			.map((b) => {
				const block = unwrapOneof(b.block ?? b);
				if (block?.key === 'text') return (block.value?.text as string) ?? '';
				return '';
			})
			.join('');
	}

	function getReasoningContent(node: BaseMessageWithSibling): string {
		return getContentBlocks(node)
			.map((b) => {
				const block = unwrapOneof(b.block ?? b);
				if (block?.key === 'reasoning') return (block.value?.reasoning as string) ?? '';
				return '';
			})
			.join('');
	}

	function isUser(node: BaseMessageWithSibling): boolean {
		const inner = unwrapOneof((node.message as Record<string, unknown>)?.message);
		return inner?.key === 'human';
	}

	function getMessageId(node: BaseMessageWithSibling): string | undefined {
		const inner = unwrapOneof((node.message as Record<string, unknown>)?.message);
		return (inner?.value?.id as string) ?? undefined;
	}

	function getSiblingIndex(node: BaseMessageWithSibling): number {
		const record = node as Record<string, unknown>;
		return (record.siblingIndex ?? record.sibling_index ?? 0) as number;
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

{#snippet siblingNav(node: BaseMessageWithSibling)}
	{#if node.children.length > 1}
		{@const activeId = getMessageId(node)}
		<Message.Action
			tooltip="Previous"
			onclick={() => {
				if (activeId && chatService.activeThreadId)
					chatService.switchBranch(chatService.activeThreadId, activeId, -1);
			}}
		>
			<ChevronLeftIcon />
		</Message.Action>
		<span class="text-muted-foreground flex items-center text-xs">
			{getSiblingIndex(node) + 1} / {node.children.length}
		</span>
		<Message.Action
			tooltip="Next"
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
		{#if chatService.activeThread?.messages.length === 0 && chatService.loading}
			{#each Array(4) as _, i}
				<div
					class="flex w-full max-w-[95%] flex-col gap-2 {i % 2 === 0
						? 'ml-auto items-end'
						: 'items-start'}"
				>
					<div class="flex flex-col gap-2 rounded-lg px-4 py-3">
						<Skeleton
							class="bg-muted h-4 w-48"
							style="background-image: linear-gradient(110deg, transparent 25%, var(--muted-foreground) 37%, transparent 63%);"
						/>
						<Skeleton
							class="bg-muted h-4 w-36"
							style="background-image: linear-gradient(110deg, transparent 25%, var(--muted-foreground) 37%, transparent 63%);"
						/>
						{#if i % 2 === 1}
							<Skeleton
								class="bg-muted h-4 w-56"
								style="background-image: linear-gradient(110deg, transparent 25%, var(--muted-foreground) 37%, transparent 63%);"
							/>
						{/if}
					</div>
				</div>
			{/each}
		{:else if chatService.activeThread?.messages.length === 0}
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
		{#each chatService.activeThread?.messages as node, i}
			{@const content = getTextContent(node)}
			{@const user = isUser(node)}
			{@const reasoning = getReasoningContent(node)}
			{@const messageId = getMessageId(node)}
			{@const isStreaming =
				!user &&
				i === (chatService.activeThread?.messages.length ?? 0) - 1 &&
				chatService.streaming}
			{#if content.length > 0 || !user}
				<Message.Root from={user ? 'user' : 'assistant'} data-message-id={messageId}>
					{#if reasoning}
						<Reasoning.Root>
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
							{:else if !reasoning}
								<Shimmer>Thinking</Shimmer>
							{/if}
						</Message.Content>
					{/if}
					{#if user && editingId !== messageId && !chatService.streaming}
						<Message.Actions class="self-end">
							{@render siblingNav(node)}
							{#if onCopy}
								<Message.Action
									tooltip="Copy"
									onclick={() => handleCopy(content, messageId!)}
								>
									{#if copiedId === messageId}
										<CheckIcon />
									{:else}
										<CopyIcon />
									{/if}
								</Message.Action>
							{/if}
							{#if onEdit}
								<Message.Action
									tooltip="Edit"
									onclick={() => startEdit(messageId!, content)}
								>
									<PencilIcon />
								</Message.Action>
							{/if}
						</Message.Actions>
					{/if}
					{#if !user && content.trim().length > 0 && !isStreaming}
						<Message.Actions>
							{@render siblingNav(node)}
							{#if onCopy}
								<Message.Action
									tooltip="Copy"
									onclick={() => handleCopy(content, messageId!)}
								>
									{#if copiedId === messageId}
										<CheckIcon />
									{:else}
										<CopyIcon />
									{/if}
								</Message.Action>
							{/if}
						</Message.Actions>
					{/if}
				</Message.Root>
			{/if}
		{/each}
	</Conversation.Content>
</Conversation.Root>
