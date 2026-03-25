<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { MESSAGE_SERVICE } from '$lib/services/message-service.svelte.js';
	import { THREAD_SERVICE } from '$lib/services/thread-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Attachment from '@eurora/ui/components/ai-elements/attachments/index';
	import * as Conversation from '@eurora/ui/components/ai-elements/conversation/index';
	import * as Message from '@eurora/ui/components/ai-elements/message/index';
	import * as PromptInput from '@eurora/ui/components/ai-elements/prompt-input/index';
	import * as Reasoning from '@eurora/ui/components/ai-elements/reasoning/index';
	import { Shimmer } from '@eurora/ui/components/ai-elements/shimmer/index';
	import * as Suggestion from '@eurora/ui/components/ai-elements/suggestion/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import { Skeleton } from '@eurora/ui/components/skeleton/index';
	import { MessageGraph } from '@eurora/ui/custom-components/message-graph/index';
	import ArrowUpCircleIcon from '@lucide/svelte/icons/arrow-up-circle';
	import CheckIcon from '@lucide/svelte/icons/check';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import PencilIcon from '@lucide/svelte/icons/pencil';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { open } from '@tauri-apps/plugin-shell';
	import { onDestroy, onMount, tick } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type {
		Query,
		ContextChip,
		TimelineAppEvent,
		MessageView,
	} from '$lib/bindings/bindings.js';
	import type {
		PromptInputMessage,
		ChatStatus,
	} from '@eurora/ui/components/ai-elements/prompt-input/index';

	let { data } = $props();

	let copiedMessageId = $state<string | null>(null);
	let editingIndex = $state<number | null>(null);
	let editText = $state('');
	let editTextarea = $state<HTMLTextAreaElement | null>(null);

	async function copyMessageContent(content: string, messageIndex: number) {
		await writeText(content);

		const id = String(messageIndex);
		copiedMessageId = id;
		setTimeout(() => {
			if (copiedMessageId === id) copiedMessageId = null;
		}, 2000);
	}

	let taurpc = inject(TAURPC_SERVICE);
	let messageService = inject(MESSAGE_SERVICE);
	let threadService = inject(THREAD_SERVICE);
	let chatStatus = $state<ChatStatus>('ready');
	let assets = $state<ContextChip[]>([]);
	let latestTimelineItem = $state<TimelineAppEvent | null>(null);
	let tokenLimitMessages = $state(new Set<number>());
	let upgradeLoading = $state(false);

	function isTokenLimitError(error: unknown): boolean {
		return String(error).includes('Monthly token limit reached');
	}

	async function handleUpgrade() {
		if (upgradeLoading) return;
		upgradeLoading = true;
		try {
			const url = await taurpc.payment.create_checkout_url();
			await open(url);
			goto('/no-access/upgrade');
		} catch (e) {
			toast.error(`Failed to start checkout: ${e}`);
			upgradeLoading = false;
		}
	}

	const threadId = $derived(data.threadId);
	const threadData = $derived(threadId ? messageService.getThread(threadId) : null);
	const messages = $derived(threadData?.messages ?? []);
	const treeNodes = $derived(threadData?.treeNodes ?? []);
	const treeHasMore = $derived(threadData?.treeHasMore ?? false);
	const treeLoading = $derived(threadData?.treeLoading ?? false);
	const threadTitle = $derived(
		threadService.threads.find((t) => t.id === threadId)?.title ?? 'New Chat',
	);
	const messagesLoading = $derived(threadData?.loading ?? false);
	const reasoningData = $derived(threadData?.reasoningData ?? {});
	const showSuggestions = $derived(messages.length === 0 && assets.length === 0);

	const activeMessageIds = $derived(
		new Set(messages.map((m) => m.id).filter((id): id is string => id !== null)),
	);

	$effect(() => {
		messageService.viewModeVisible = messages.length > 0;
	});

	$effect(() => {
		if (messageService.viewMode === 'graph' && threadId) {
			messageService.ensureTreeLoaded(threadId);
		}
	});

	function handleLoadMoreLevels() {
		if (!threadId) return;
		messageService.loadMoreTreeLevels(threadId);
	}

	onDestroy(() => {
		messageService.viewModeVisible = false;
	});

	function handleSwitchBranch(messageId: string, direction: number) {
		if (!threadId) return;
		messageService.switchBranch(threadId, messageId, direction).catch((error) => {
			toast.error(`Failed to switch branch: ${error}`);
		});
	}

	async function handleGraphNodeDblClick(messageId: string) {
		if (!threadId) return;
		try {
			await messageService.navigateToMessage(threadId, messageId);
			await tick();
			const el = document.querySelector(`[data-message-id="${CSS.escape(messageId)}"]`);
			el?.scrollIntoView({ behavior: 'auto', block: 'center' });
		} catch (error) {
			toast.error(`Failed to navigate to message: ${error}`);
		}
	}

	$effect(() => {
		if (threadData?.streaming) {
			chatStatus = 'streaming';
		} else if (chatStatus === 'streaming') {
			chatStatus = 'ready';
		}
	});

	const suggestions = [
		'What are the latest trends in AI?',
		'How does machine learning work?',
		'Explain quantum computing',
		'Best practices for React development',
		'Tell me about TypeScript benefits',
		'How to optimize database queries?',
		'What is the difference between SQL and NoSQL?',
		'Explain cloud computing basics',
	];

	function handleSuggestionClick(suggestion: string) {
		chatStatus = 'submitted';
		sendQuery(suggestion).catch((error) => handleQueryError(error));
	}

	function removeAsset(id: string) {
		assets = assets.filter((a) => a.id !== id);
	}

	function middleTruncate(text: string, maxTokens = 5): string {
		const parts = text.split(/([^a-zA-Z0-9]+)/);
		const words = parts.filter((p) => /[a-zA-Z0-9]/.test(p));
		if (words.length <= maxTokens * 2) return text;

		let start = '';
		let count = 0;
		for (const part of parts) {
			if (/[a-zA-Z0-9]/.test(part)) count++;
			if (count > maxTokens) break;
			start += part;
		}

		let end = '';
		count = 0;
		for (let i = parts.length - 1; i >= 0; i--) {
			if (/[a-zA-Z0-9]/.test(parts[i])) count++;
			if (count > maxTokens) break;
			end = parts[i] + end;
		}

		return start + '(...)' + end;
	}

	onMount(() => {
		taurpc.timeline.new_assets_event.on((chips) => {
			assets = chips;
		});

		taurpc.timeline.new_app_event.on((e) => {
			latestTimelineItem = e;
		});
	});

	function getMessageContent(message: any): string {
		if (message.type === 'remove') {
			return '';
		}
		const content = message.content;
		if (typeof content === 'string') {
			return content;
		}
		if (Array.isArray(content)) {
			return content
				.filter((part): part is { type: 'text'; text: string } => part.type === 'text')
				.map((part) => part.text)
				.join(' ');
		}
		return '';
	}

	function isUserMessage(message: any): boolean {
		return message.role === 'human';
	}

	function handleQueryError(error: unknown) {
		if (isTokenLimitError(error)) {
			const msgs = threadData?.messages;
			if (msgs) {
				const lastIndex = msgs.length - 1;
				const lastMsg = msgs[lastIndex];
				if (lastMsg && lastMsg.role === 'ai') {
					lastMsg.content =
						"You've reached your free trial token limit for this month. Upgrade to Pro for unlimited access.";
					tokenLimitMessages = new Set([...tokenLimitMessages, lastIndex]);
				}
			}
			chatStatus = 'ready';
		} else {
			chatStatus = 'error';
			toast.error(String(error), {
				duration: Infinity,
				cancel: { label: 'Ok', onClick: () => {} },
			});
		}
	}

	function handleSubmit(message: PromptInputMessage) {
		const text = message.text.trim();
		if (!text) return;

		chatStatus = 'submitted';
		const currentAssets = [...assets];
		const assetIds = currentAssets.map((a) => a.id);
		sendQuery(text, assetIds, currentAssets).catch((error) => handleQueryError(error));
	}

	async function startEdit(index: number, content: string) {
		editingIndex = index;
		editText = content;
		await tick();
		editTextarea?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
		editTextarea?.focus();
	}

	function cancelEdit() {
		editingIndex = null;
		editText = '';
	}

	function submitEdit() {
		if (editingIndex === null || !threadId) return;
		const text = editText.trim();
		if (!text) return;

		const parentId = editingIndex > 0 ? (messages[editingIndex - 1]?.id ?? '') : '';
		const assetChips = messages[editingIndex]?.assets ?? [];

		chatStatus = 'submitted';
		const idx = editingIndex;
		editingIndex = null;
		editText = '';

		messageService
			.editMessage(threadId, idx, text, parentId, assetChips)
			.catch((error) => handleQueryError(error));
	}

	function handleEditKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			submitEdit();
		} else if (e.key === 'Escape') {
			cancelEdit();
		}
	}

	async function sendQuery(
		text: string,
		assetIds: string[] = [],
		contextAssets: ContextChip[] = [],
	): Promise<void> {
		const query: Query = { text, assets: assetIds, parent_message_id: null };
		let targetThreadId = threadId;

		if (!targetThreadId) {
			const created = await taurpc.thread.create();
			targetThreadId = created.id;
			if (!targetThreadId) {
				throw new Error('Failed to create thread');
			}
			threadService.addThread(created);
			await goto(`/${targetThreadId}`, { replaceState: true });
		}

		const assetChips = contextAssets.map((a) => ({
			id: a.id,
			name: a.name,
			icon: a.icon,
		}));

		const streamPromise = messageService.sendMessage(
			targetThreadId,
			query,
			assetChips.length ? assetChips : undefined,
		);

		taurpc.thread.generate_title(targetThreadId, text).then((updated) => {
			threadService.updateThread(updated);
		});

		await streamPromise;
		chatStatus = 'ready';
	}
</script>

{#snippet siblingNav(message: MessageView)}
	{#if message.sibling_count > 1 && message.id}
		<Message.Action tooltip="Previous" onclick={() => handleSwitchBranch(message.id!, -1)}>
			<ChevronLeftIcon />
		</Message.Action>
		<span class="text-muted-foreground flex items-center text-xs">
			{message.sibling_index + 1} / {message.sibling_count}
		</span>
		<Message.Action tooltip="Next" onclick={() => handleSwitchBranch(message.id!, 1)}>
			<ChevronRightIcon />
		</Message.Action>
	{/if}
{/snippet}

<div class="flex h-full flex-col overflow-hidden">
	{#if messageService.viewMode === 'graph' && (messages.length > 0 || messagesLoading)}
		<div class="min-h-0 flex-1">
			<MessageGraph
				{treeNodes}
				{activeMessageIds}
				startLabel={threadTitle}
				loading={messagesLoading || treeLoading}
				hasMoreLevels={treeHasMore}
				loadingMoreLevels={treeLoading}
				onmessagedblclick={handleGraphNodeDblClick}
				onloadmorelevels={handleLoadMoreLevels}
			/>
		</div>
	{:else}
		<Conversation.Root class="min-h-0 flex-1">
			<Conversation.Content>
				{#if messages.length === 0 && messagesLoading}
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
				{:else if messages.length === 0}
					<Empty.Root>
						<Empty.Header>
							{#if latestTimelineItem?.icon_base64}
								<Empty.Title>Currently on</Empty.Title>
								<Empty.Media variant="icon" class="bg-transparent">
									<img
										src={latestTimelineItem.icon_base64}
										alt=""
										class="size-full"
									/>
								</Empty.Media>
							{:else}
								<Empty.Title>No messages yet</Empty.Title>
							{/if}
						</Empty.Header>
					</Empty.Root>
				{/if}
				{#each messages as message, i}
					{@const content = getMessageContent(message)}
					{@const isUser = isUserMessage(message)}
					{@const reasoning = reasoningData[i]}
					{#if content.length > 0 || !isUser}
						<Message.Root
							from={isUser ? 'user' : 'assistant'}
							data-message-id={message.id}
						>
							{#if isUser && message.assets?.length}
								<Attachment.Root variant="inline" class="ml-auto">
									{#each message.assets as asset (asset.id)}
										<Attachment.Item
											data={{
												type: 'file',
												id: asset.id,
												filename: middleTruncate(asset.name),
											}}
										>
											<Attachment.Preview />
											<Attachment.Info />
										</Attachment.Item>
									{/each}
								</Attachment.Root>
							{/if}
							{#if reasoning}
								<Reasoning.Root
									isStreaming={reasoning.isStreaming}
									duration={reasoning.duration}
								>
									<Reasoning.Trigger />
									<Reasoning.Content children={reasoning.content} />
								</Reasoning.Root>
							{/if}
							{#if isUser && editingIndex === i}
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
							{@const isStreaming =
								!isUser && i === messages.length - 1 && chatStatus !== 'ready'}
							{#if isUser && editingIndex !== i && chatStatus === 'ready'}
								<Message.Actions class="self-end">
									{@render siblingNav(message)}
									<Message.Action
										tooltip="Copy"
										onclick={() => copyMessageContent(content, i)}
									>
										{#if copiedMessageId === String(i)}
											<CheckIcon />
										{:else}
											<CopyIcon />
										{/if}
									</Message.Action>
									<Message.Action
										tooltip="Edit"
										onclick={() => startEdit(i, content)}
									>
										<PencilIcon />
									</Message.Action>
								</Message.Actions>
							{/if}
							{#if !isUser && content.trim().length > 0 && !isStreaming}
								<Message.Actions>
									{@render siblingNav(message)}
									{#if tokenLimitMessages.has(i)}
										<Message.Action
											tooltip="Upgrade Plan"
											onclick={handleUpgrade}
											variant="default"
											size="lg"
											disabled={upgradeLoading}
										>
											{#if upgradeLoading}
												<Loader2Icon class="animate-spin" />
											{:else}
												Upgrade Plan
												<ArrowUpCircleIcon />
											{/if}
										</Message.Action>
									{:else}
										<Message.Action
											tooltip="Copy"
											onclick={() => copyMessageContent(content, i)}
										>
											{#if copiedMessageId === String(i)}
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
	{/if}
	<div class="grid shrink-0 gap-4">
		{#if showSuggestions}
			<Suggestion.Root class="px-4">
				{#each suggestions as suggestion}
					<Suggestion.Item {suggestion} onclick={handleSuggestionClick} />
				{/each}
			</Suggestion.Root>
		{/if}
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="w-full cursor-text px-4 pb-4"
			onclick={(e) => {
				if (
					e.target === e.currentTarget ||
					!(e.target as HTMLElement).closest('textarea, button, a, input')
				) {
					const textarea = e.currentTarget.querySelector('textarea');
					textarea?.focus();
				}
			}}
		>
			<PromptInput.Root onSubmit={handleSubmit}>
				{#if assets.length > 0}
					<PromptInput.Header>
						<Attachment.Root variant="inline">
							{#each assets as asset (asset.id)}
								<Attachment.Item
									data={{
										type: 'file',
										id: asset.id,
										filename: middleTruncate(asset.name),
									}}
									onRemove={() => removeAsset(asset.id)}
								>
									<Attachment.Preview />
									<Attachment.Info />
									<Attachment.Remove />
								</Attachment.Item>
							{/each}
						</Attachment.Root>
					</PromptInput.Header>
				{/if}
				<PromptInput.Body>
					<PromptInput.Textarea placeholder="What can I help you with?" />
				</PromptInput.Body>
				<PromptInput.Footer class="justify-end">
					<!-- <PromptInput.Tools>
						<PromptInput.Button
							size="sm"
							onclick={() => (useWebSearch = !useWebSearch)}
							variant={useWebSearch ? 'default' : 'ghost'}
						>
							<GlobeIcon size={16} />
							<span>Search</span>
						</PromptInput.Button>
					</PromptInput.Tools> -->
					<PromptInput.Submit status={chatStatus} />
				</PromptInput.Footer>
			</PromptInput.Root>
		</div>
	</div>
</div>
