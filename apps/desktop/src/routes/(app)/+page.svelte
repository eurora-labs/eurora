<script lang="ts">
	import 'katex/dist/katex.min.css';
	import {
		type ResponseChunk,
		type Query,
		type MessageView,
		type ThreadView,
	} from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { executeCommand } from '$lib/commands.js';
	import Katex from '$lib/components/katex.svelte';
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
	import { ScrollArea } from '@eurora/ui/components/scroll-area/index';
	import * as Chat from '@eurora/ui/custom-components/chat/index';
	import { Thinking } from '@eurora/ui/custom-components/thinking/index';
	import { onMount } from 'svelte';

	let thread = $state<ThreadView | null>(null);
	let messages = $state<MessageView[]>([]);
	let taurpc = inject(TAURPC_SERVICE);

	let editorRef: ProsemirrorEditor | undefined = $state();
	let chatRef = $state<Chat.Root | null>(null);

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
				console.error('Error:', error);
			}
		}
	}

	async function askQuestion(query: QueryAssets): Promise<void> {
		try {
			const tauRpcQuery: Query = {
				text: query.text,
				assets: query.assets,
			};
			const aiMessage: MessageView = {
				id: null,
				role: 'ai',
				content: '',
			};
			messages.push(aiMessage);
			const agentMessage = messages.at(-1);

			function onEvent(response: ResponseChunk) {
				if (agentMessage && agentMessage.role === 'ai') {
					agentMessage.content += response.chunk;
				}

				chatRef?.scrollToBottom();
			}

			await taurpc.chat.send_query(thread?.id ?? null, onEvent, tauRpcQuery);
		} catch (error) {
			console.error('Failed to get answer:', error);
		}
	}
</script>

<div class="w-full h-full">
	{#if messages.length > 0}
		<ScrollArea class="h-full w-full px-6">
			<Chat.Root
				bind:this={chatRef}
				class="w-full h-full flex flex-col gap-4 overflow-hidden pb-28"
			>
				{#each messages as message}
					{@const content = getMessageContent(message)}
					{@const isUser = isUserMessage(message)}
					{#if content.length > 0 || !isUser}
						<Chat.Message
							variant={isUser ? 'default' : 'assistant'}
							finishRendering={() => {}}
						>
							<Chat.MessageContent>
								{#if content.trim().length > 0}
									<Katex math={content} />
								{:else}
									<Thinking class="text-primary/60" />
								{/if}
							</Chat.MessageContent>
						</Chat.Message>
					{/if}
				{/each}
			</Chat.Root>
		</ScrollArea>
	{/if}

	<div
		class={[
			'flex justify-center',
			messages.length > 0
				? 'fixed bottom-4 left-[var(--sidebar-width)] right-0 z-10'
				: 'h-full items-center',
		]}
	>
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
