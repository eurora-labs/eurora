<script lang="ts" module>
	import type { Snippet } from 'svelte';

	interface Props {
		emptyState?: Snippet;
		onCopy?: (content: string) => void;
		onEdit?: (messageId: string, newText: string) => void;
		onRegenerate?: (messageId: string) => void;
	}
</script>

<script lang="ts">
	import MessageItem from '$lib/components/MessageItem.svelte';
	import { CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { getReasoningFromMessage } from '$lib/utils/asset-chips.js';
	import { getTextContent, messageId } from '$lib/utils/message-content.js';
	import { useIdleRef } from '$lib/utils/throttled.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Conversation from '@eurora/ui/components/ai-elements/conversation/index';
	import { initStickToBottomContext } from '@eurora/ui/components/ai-elements/conversation/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import { Skeleton } from '@eurora/ui/components/skeleton/index';
	import type { MessageNode } from '$lib/models/messages/index.js';
	import type { BranchDirection } from '$lib/services/thread/thread-service.js';

	let { emptyState, onCopy, onEdit, onRegenerate }: Props = $props();

	const chatService = inject(CHAT_SERVICE);
	const scrollContext = initStickToBottomContext();

	let prevThreadId: string | undefined;
	let prevStreamingId: string | null | undefined;

	// Streaming nodes mutate their text on every chunk. Markdown reparse +
	// token tree rebuild for the trailing message is the dominant main-thread
	// cost — heavy enough to peg a CPU if it runs at chunk rate. Defer those
	// renders to browser idle time: `IdleRef.current` only updates when the
	// browser has spare cycles, and newer chunks supersede pending idle
	// updates so intermediate snapshots are dropped rather than queued. When
	// the stream ends, MessageItem switches to reading `getTextContent(node)`
	// directly, so the streaming signal can drop to '' without a visible gap.
	const streamingId = $derived(chatService.activeThread?.streamingMessageId ?? null);
	const isAnyStreaming = $derived(streamingId !== null);
	const streamingNode = $derived.by<MessageNode | null>(() => {
		const id = streamingId;
		if (!id) return null;
		// The streaming placeholder is always the most recently appended node;
		// fall back to a search only if that invariant is somehow violated.
		const messages = chatService.activeThread?.messages;
		if (!messages || messages.length === 0) return null;
		const last = messages[messages.length - 1];
		if (last.message.id === id) return last;
		return messages.find((n) => n.message.id === id) ?? null;
	});

	const idleStreamingContent = useIdleRef({
		source: () => (streamingNode ? getTextContent(streamingNode) : ''),
		isLive: () => streamingId !== null,
	});
	const idleStreamingReasoning = useIdleRef({
		source: () => (streamingNode ? getReasoningFromMessage(streamingNode.message) : ''),
		isLive: () => streamingId !== null,
	});

	$effect(() => {
		const threadId = chatService.activeThreadId;
		const currentStreamingId = streamingId;

		const threadChanged = threadId !== prevThreadId;
		const streamingStarted =
			currentStreamingId !== null && currentStreamingId !== prevStreamingId;

		if (threadChanged || streamingStarted) {
			scrollContext.reengageAutoScroll();
		}

		prevThreadId = threadId;
		prevStreamingId = currentStreamingId;
	});

	function handleSwitchBranch(id: string, direction: BranchDirection) {
		const threadId = chatService.activeThreadId;
		if (!threadId) return;
		void chatService.switchBranch(threadId, id, direction);
	}
</script>

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
		{#each chatService.activeThread?.messages ?? [] as node (messageId(node))}
			<MessageItem
				{node}
				isStreaming={streamingId === messageId(node)}
				{isAnyStreaming}
				streamingContent={idleStreamingContent.current}
				streamingReasoning={idleStreamingReasoning.current}
				{onCopy}
				{onEdit}
				{onRegenerate}
				onSwitchBranch={handleSwitchBranch}
			/>
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
