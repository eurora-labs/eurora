<script lang="ts">
	import {
		type ResponseChunk,
		type Query,
		type MessageView,
		type ThreadView,
	} from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { executeCommand } from '$lib/commands.js';
	import {
		Editor as ProsemirrorEditor,
		type SveltePMExtension,
	} from '@eurora/prosemirror-core/index';
	import {
		processQuery,
		clearQuery,
		clearExtensionNodes,
		type QueryAssets,
	} from '@eurora/prosemirror-core/util';
	import { extensionFactory, registerCoreExtensions } from '@eurora/prosemirror-factory/index';
	import * as Launcher from '@eurora/prosemirror-view/launcher';
	import { inject } from '@eurora/shared/context';
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
	import { Shimmer } from '@eurora/ui/components/ai-elements/shimmer/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';
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

	let editorRef: ProsemirrorEditor | undefined = $state();

	registerCoreExtensions();
	let searchQuery = $state({
		text: '',
		extensions: getExtensions(),
	});

	function getExtensions(): SveltePMExtension[] {
		return [
			extensionFactory.getExtension('7c7b59bb-d44d-431a-9f4d-64240172e092'),
			extensionFactory.getExtension('309f0906-d48c-4439-9751-7bcf915cdfc5'),
			extensionFactory.getExtension('2c434895-d32c-485f-8525-c4394863b83a'),
		];
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

		taurpc.timeline.new_assets_event.on((assets) => {
			if (!editorRef) return false;
			clearExtensionNodes(editorRef);
			assets.forEach((command) => {
				executeCommand(editorRef!, command);
			});
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

	async function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			try {
				if (!editorRef) {
					console.error('No editor ref found');
					return;
				}
				const query = processQuery(editorRef);
				messages.push({
					id: null,
					role: 'human',
					content: query.text,
				});
				searchQuery.text = '';
				clearQuery(editorRef);
				await askQuestion(query);
			} catch (error) {
				messages.splice(-2);
				toast.error(String(error), {
					duration: Infinity,
					cancel: { label: 'Ok', onClick: () => {} },
				});
			}
		}
	}

	async function askQuestion(query: QueryAssets): Promise<void> {
		const tauRpcQuery: Query = {
			text: query.text,
			assets: query.assets,
		};

		let agentMessage: MessageView | undefined;
		messages.push({
			id: null,
			role: 'ai',
			content: '',
		});

		function onEvent(response: ResponseChunk) {
			if (!agentMessage) {
				agentMessage = messages.at(-1);
			}

			if (agentMessage && agentMessage.role === 'ai') {
				agentMessage.content += response.chunk;
			}
		}

		await taurpc.chat.send_query(thread?.id ?? null, onEvent, tauRpcQuery);
	}
</script>

<div class="flex h-svh flex-col overflow-hidden">
	<Conversation class="min-h-0 flex-1">
		{#if messages.length > 0}
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
							{#if !isUser && content.trim().length > 0}
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
		{/if}
	</Conversation>
	<div class="flex shrink-0 justify-center p-4">
		<Launcher.Root
			class="h-fit rounded-2xl shadow-none flex flex-col p-4 w-[90%] bg-card text-card-foreground border border-border"
		>
			<Launcher.Input
				placeholder="What can I help you with?"
				class="min-h-25 h-fit w-full text-[24px]"
				bind:query={searchQuery}
				bind:editorRef
				onkeydown={handleKeydown}
			/>
		</Launcher.Root>
	</div>
</div>
