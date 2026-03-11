<script lang="ts">
	import { goto } from '$app/navigation';
	import { type Query, type ContextChip, type TimelineAppEvent } from '$lib/bindings/bindings.js';
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
	import * as Empty from '@eurora/ui/components/empty/index';
	import ArrowUpCircleIcon from '@lucide/svelte/icons/arrow-up-circle';
	import Loader2Icon from '@lucide/svelte/icons/loader-2';
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { open } from '@tauri-apps/plugin-shell';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type {
		PromptInputMessage,
		ChatStatus,
	} from '@eurora/ui/components/ai-elements/prompt-input/index';

	let { data } = $props();

	let copiedMessageId = $state<string | null>(null);

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
	const reasoningData = $derived(threadData?.reasoningData ?? {});
	const showSuggestions = $derived(messages.length === 0 && assets.length === 0);

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
		const assetIds = assets.map((a) => a.id);
		sendQuery(text, assetIds).catch((error) => handleQueryError(error));
	}

	async function sendQuery(text: string, assetIds: string[] = []): Promise<void> {
		const query: Query = { text, assets: assetIds };
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

		const streamPromise = messageService.sendMessage(targetThreadId, query);

		taurpc.thread.generate_title(targetThreadId, text).then((updated) => {
			threadService.updateThread(updated);
		});

		await streamPromise;
		chatStatus = 'ready';
	}
</script>

<div class="flex h-full flex-col overflow-hidden">
	<Conversation.Root class="min-h-0 flex-1">
		<Conversation.Content>
			{#if messages.length === 0}
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
					<Message.Root from={isUser ? 'user' : 'assistant'}>
						{#if reasoning}
							<Reasoning.Root
								isStreaming={reasoning.isStreaming}
								duration={reasoning.duration}
							>
								<Reasoning.Trigger />
								<Reasoning.Content children={reasoning.content} />
							</Reasoning.Root>
						{/if}
						<Message.Content>
							{#if content.trim().length > 0}
								<Message.Response {content} />
							{:else if !reasoning}
								<Shimmer>Thinking</Shimmer>
							{/if}
						</Message.Content>
						{@const isStreaming =
							!isUser && i === messages.length - 1 && chatStatus !== 'ready'}
						{#if !isUser && content.trim().length > 0 && !isStreaming}
							<Message.Actions>
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
	<div class="grid shrink-0 gap-4">
		{#if showSuggestions}
			<Suggestion.Root class="px-4">
				{#each suggestions as suggestion}
					<Suggestion.Item {suggestion} onclick={handleSuggestionClick} />
				{/each}
			</Suggestion.Root>
		{/if}
		<div class="w-full px-4 pb-4">
			<PromptInput.Root onSubmit={handleSubmit}>
				{#if assets.length > 0}
					<PromptInput.Header>
						<Attachment.Root variant="inline">
							{#each assets as asset (asset.id)}
								<Attachment.Item
									data={{ type: 'file', id: asset.id, filename: asset.name }}
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
