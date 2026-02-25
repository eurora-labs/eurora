<script lang="ts">
	import {
		type ResponseChunk,
		type Query,
		type MessageView,
		type ThreadView,
		type ContextChip,
	} from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import {
		Attachments,
		Attachment,
		AttachmentPreview,
		AttachmentInfo,
		AttachmentRemove,
	} from '@eurora/ui/components/ai-elements/attachments/index';
	import {
		Conversation,
		ConversationContent,
	} from '@eurora/ui/components/ai-elements/conversation/index';
	import {
		Message,
		MessageContent,
		MessageActions,
		MessageAction,
		MessageResponse,
	} from '@eurora/ui/components/ai-elements/message/index';
	import {
		PromptInput,
		PromptInputBody,
		PromptInputTextarea,
		PromptInputHeader,
		PromptInputFooter,
		PromptInputTools,
		PromptInputButton,
		PromptInputSubmit,
		type PromptInputMessage,
		type ChatStatus,
	} from '@eurora/ui/components/ai-elements/prompt-input/index';
	import { Shimmer } from '@eurora/ui/components/ai-elements/shimmer/index';
	import { Suggestions, Suggestion } from '@eurora/ui/components/ai-elements/suggestion/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	let copiedMessageId = $state<string | null>(null);

	async function copyMessageContent(content: string, messageIndex: number) {
		await writeText(content);

		const id = String(messageIndex);
		copiedMessageId = id;
		setTimeout(() => {
			if (copiedMessageId === id) copiedMessageId = null;
		}, 2000);
	}

	let thread = $state<ThreadView | null>(null);
	let messages = $state<MessageView[]>([]);
	let taurpc = inject(TAURPC_SERVICE);
	let chatStatus = $state<ChatStatus>('ready');
	let useWebSearch = $state(true);
	let assets = $state<ContextChip[]>([]);

	const showSuggestions = $derived(messages.length === 0 && assets.length === 0);

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
		messages.push({
			id: null,
			role: 'human',
			content: suggestion,
		});

		chatStatus = 'submitted';
		askQuestion(suggestion).catch((error) => {
			messages.splice(-2);
			chatStatus = 'error';
			toast.error(String(error), {
				duration: Infinity,
				cancel: { label: 'Ok', onClick: () => {} },
			});
		});
	}

	function removeAsset(id: string) {
		assets = assets.filter((a) => a.id !== id);
	}

	onMount(() => {
		taurpc.thread.current_thread_changed.on((new_conv) => {
			thread = new_conv;

			if (!new_conv.id) {
				messages.splice(0, messages.length);
				return;
			}

			taurpc.thread.get_messages(new_conv.id, 50, 0).then((response) => {
				messages = response;
			});
		});

		taurpc.thread.new_thread_added.on((new_thread) => {
			thread = new_thread;
		});

		taurpc.timeline.new_assets_event.on((chips) => {
			assets = chips;
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

	function handleSubmit(message: PromptInputMessage) {
		const text = message.text.trim();
		if (!text) return;

		const assetIds = assets.map((a) => a.id);

		messages.push({
			id: null,
			role: 'human',
			content: text,
		});

		chatStatus = 'submitted';
		askQuestion(text, assetIds).catch((error) => {
			messages.splice(-2);
			chatStatus = 'error';
			toast.error(String(error), {
				duration: Infinity,
				cancel: { label: 'Ok', onClick: () => {} },
			});
		});
	}

	async function askQuestion(text: string, assetIds: string[] = []): Promise<void> {
		const tauRpcQuery: Query = {
			text,
			assets: assetIds,
		};

		let agentMessage: MessageView | undefined;
		messages.push({
			id: null,
			role: 'ai',
			content: '',
		});

		chatStatus = 'streaming';

		function onEvent(response: ResponseChunk) {
			if (!agentMessage) {
				agentMessage = messages.at(-1);
			}

			if (agentMessage && agentMessage.role === 'ai') {
				agentMessage.content += response.chunk;
			}
		}

		await taurpc.chat.send_query(thread?.id ?? null, onEvent, tauRpcQuery);
		chatStatus = 'ready';
	}
</script>

<div class="flex h-full flex-col overflow-hidden">
	<Conversation class="min-h-0 flex-1">
		<ConversationContent>
			{#each messages as message, i}
				{@const content = getMessageContent(message)}
				{@const isUser = isUserMessage(message)}
				{#if content.length > 0 || !isUser}
					<Message from={isUser ? 'user' : 'assistant'}>
						<MessageContent>
							{#if content.trim().length > 0}
								<MessageResponse {content} />
							{:else}
								<Shimmer>Thinking</Shimmer>
							{/if}
						</MessageContent>
						{@const isStreaming =
							!isUser && i === messages.length - 1 && chatStatus !== 'ready'}
						{#if !isUser && content.trim().length > 0 && !isStreaming}
							<MessageActions>
								<MessageAction
									tooltip="Copy"
									onclick={() => copyMessageContent(content, i)}
								>
									{#if copiedMessageId === String(i)}
										<CheckIcon />
									{:else}
										<CopyIcon />
									{/if}
								</MessageAction>
							</MessageActions>
						{/if}
					</Message>
				{/if}
			{/each}
		</ConversationContent>
	</Conversation>
	<div class="grid shrink-0 gap-4">
		{#if showSuggestions}
			<Suggestions class="px-4">
				{#each suggestions as suggestion}
					<Suggestion {suggestion} onclick={handleSuggestionClick} />
				{/each}
			</Suggestions>
		{/if}
		<div class="w-full px-4 pb-4">
			<PromptInput onSubmit={handleSubmit}>
				<PromptInputHeader>
					{#if assets.length > 0}
						<Attachments variant="inline">
							{#each assets as asset (asset.id)}
								<Attachment
									data={{ type: 'file', id: asset.id, filename: asset.name }}
									onRemove={() => removeAsset(asset.id)}
								>
									<AttachmentPreview />
									<AttachmentInfo />
									<AttachmentRemove />
								</Attachment>
							{/each}
						</Attachments>
					{/if}
				</PromptInputHeader>
				<PromptInputBody>
					<PromptInputTextarea placeholder="What can I help you with?" />
				</PromptInputBody>
				<PromptInputFooter>
					<PromptInputTools>
						<PromptInputButton
							size="sm"
							onclick={() => (useWebSearch = !useWebSearch)}
							variant={useWebSearch ? 'default' : 'ghost'}
						>
							<GlobeIcon size={16} />
							<span>Search</span>
						</PromptInputButton>
					</PromptInputTools>
					<PromptInputSubmit status={chatStatus} />
				</PromptInputFooter>
			</PromptInput>
		</div>
	</div>
</div>
