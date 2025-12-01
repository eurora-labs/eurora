<script lang="ts">
	import 'katex/dist/katex.min.css';
	import Katex from '$lib/components/katex.svelte';
	import { onMount } from 'svelte';
	import {
		type ResponseChunk,
		type Query,
		// type ContextChip,
		type Message,
		type Conversation,
	} from '$lib/bindings/bindings.js';

	import { processQuery, clearQuery, type QueryAssets } from '@eurora/prosemirror-core/util';
	import * as Launcher from '@eurora/prosemirror-view/launcher';
	import * as Chat from '@eurora/ui/custom-components/chat/index';
	import { extensionFactory, registerCoreExtensions } from '@eurora/prosemirror-factory/index';
	import { executeCommand } from '$lib/commands.js';
	// import { extensionFactory, registerCoreExtensions } from '$lib/prosemirror/index.js';
	import { ScrollArea } from '@eurora/ui/components/scroll-area/index';
	import { inject } from '@eurora/shared/context';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';

	import {
		Editor as ProsemirrorEditor,
		type SveltePMExtension,
	} from '@eurora/prosemirror-core/index';

	let conversation = $state<Conversation | null>(null);
	let messages = $state<Message[]>([]);
	let status = $state<string>('');
	let taurpc = inject(TAURPC_SERVICE);

	let editorRef: ProsemirrorEditor | undefined = $state();
	let chatRef = $state<Chat.Root | null>(null);

	registerCoreExtensions();
	let searchQuery = $state({
		text: '',
		extensions: [
			// extensionFactory.getExtension('9370B14D-B61C-4CE2-BDE7-B18684E8731A'),
			extensionFactory.getExtension('7c7b59bb-d44d-431a-9f4d-64240172e092'),
			extensionFactory.getExtension('309f0906-d48c-4439-9751-7bcf915cdfc5'),
			extensionFactory.getExtension('2c434895-d32c-485f-8525-c4394863b83a'),
		] as SveltePMExtension[],
	});

	onMount(() => {
		document.addEventListener('keydown', handleEscapeKey);
		taurpc.prompt
			.get_service_name()
			.then((name: string) => {
				if (name) {
					status = 'ready';
				}
			})
			.catch(() => {
				// goto('/onboarding');
			});

		taurpc.chat.current_conversation_changed.on((new_conv) => {
			conversation = new_conv;
			console.log('New conversation changed: ', conversation);

			taurpc.personal_db.message.get(conversation.id, 5, 0).then((response) => {
				messages = response;
				console.log('messages: ', messages);
			});
		});

		taurpc.timeline.new_assets_event.on((assets) => {
			if (!editorRef) return false;
			clearQuery(editorRef);
			assets.forEach((command) => {
				executeCommand(editorRef!, command);
			});
		});

		// taurpc.timeline.new_app_event.on(async (e) => {
		// 	const intervalID = setInterval(async () => {
		// 		const success = await loadActivities();
		// 		if (success) {
		// 			clearInterval(intervalID);
		// 		}
		// 	}, 1000);
		// });
	});

	// // Function to load activities from the backend
	// async function loadActivities(): Promise<boolean> {
	// 	try {
	// 		// Note: list_activities is not yet available in TauRPC, fallback to invoke for now
	// 		const result: ContextChip[] = await taurpc.context_chip.get();
	// 		if (!editorRef) return false;
	// 		result.forEach((command) => {
	// 			executeCommand(editorRef!, command);
	// 		});
	// 		return true;
	// 	} catch (error) {
	// 		console.error('Failed to load activities:', error);
	// 		return false;
	// 	}
	// }

	function handleEscapeKey(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			messages.splice(0, messages.length);
			console.log('Escape pressed: cleared messages and set conversation to NEW');
		}
	}

	// function addExampleMessages() {
	// 	messages.push(
	// 		create(ProtoChatMessageSchema, {
	// 			role: 'user',
	// 			content: 'What am I doing right now?',
	// 		}),
	// 	);

	// 	messages.push(
	// 		create(ProtoChatMessageSchema, {
	// 			role: 'system',
	// 			content:
	// 				'You are currently looking at a website called Eurora AI. What would you like to know?',
	// 		}),
	// 	);

	// 	messages.push(
	// 		create(ProtoChatMessageSchema, {
	// 			role: 'user',
	// 			content: 'How do I install it?',
	// 		}),
	// 	);
	// }

	// function handleKeydown(event: KeyboardEvent) {
	// 	if (event.key === 'Enter') {
	// 		// addExampleMessages();
	// 	}
	// }

	async function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			// await taurpc.window.resize_launcher_window(100, 1.0);

			try {
				if (!editorRef) {
					console.error('No editor ref found');
					return;
				}
				const query = processQuery(editorRef);
				messages.push({
					role: 'user',
					content: query.text,
				});
				console.log('query', query);
				searchQuery.text = '';
				clearQuery(editorRef);
				await askQuestion(query);
				// Responses will come through the event listener
			} catch (error) {
				console.error('Error:', error);
			}
		}
	}

	async function askQuestion(query: QueryAssets): Promise<void> {
		console.log('askQuestion', query);
		try {
			// Convert QueryAssets to Query type expected by TauRPC
			const tauRpcQuery: Query = {
				text: query.text,
				assets: query.assets,
			};
			// messages.push(create(ProtoChatMessageSchema, { role: 'agent', content: '' }));
			messages.push({
				role: 'assistant',
				content: '',
			});
			const agentMessage = messages.at(-1);

			const onEvent = (response: ResponseChunk) => {
				// Append chunk to the last message
				if (agentMessage) {
					agentMessage.content += response.chunk;
				}

				chatRef?.scrollToBottom();
			};

			// If no conversation is selected create a new one
			if (!conversation) {
				conversation = await taurpc.personal_db.conversation.create();
				console.log('conversation', conversation);
			}

			// Use TauRPC send_query procedure
			await taurpc.chat.send_query(conversation, onEvent, tauRpcQuery);
		} catch (error) {
			console.error('Failed to get answer:', error);
			messages.push({
				role: 'system',
				content: 'Error: Failed to get response from server' + error,
			});
		}
	}
</script>

<!-- <div class="absolute top-1/2 w-full h-fit">
	<div class=" flex flex-col justify-center items-center gap-4 w-full">
		{#if messages.length === 0}
			<h1 class="text-2xl font-bold">Eurora is ready!</h1>
			<div class="flex justify-start">
				Either press the hotkey, the Eurora Logo on the right of your screen, or start
				asking questions here.
			</div>
		{/if}
	</div>
</div> -->

<div
	class="w-full h-full flex flex-col {messages.length === 0
		? 'justify-center'
		: 'justify-end'} items-center gap-4 pb-4"
>
	{#if messages.length > 0}
		<ScrollArea
			class="w-full max-h-[calc(80vh-100px)] px-6 flex flex-col justify-end items-center gap-4"
		>
			<Chat.Root
				bind:this={chatRef}
				class="w-full h-full flex flex-col gap-4 overflow-hidden"
			>
				{#each messages as message}
					{#if typeof message.content === 'string'}
						{#if message.content.length > 0}
							<Chat.Message
								variant={message.role === 'user' ? 'default' : 'assistant'}
								finishRendering={() => {}}
							>
								<Chat.MessageContent>
									<Katex math={message.content} finishRendering={() => {}} />
								</Chat.MessageContent>
							</Chat.Message>
						{/if}
					{/if}
				{/each}
			</Chat.Root>
		</ScrollArea>
	{/if}

	<Launcher.Root
		class="h-fit rounded-[36px] shadow-none flex flex-col p-4 m-0 w-[70%] bg-gray-200"
	>
		<Launcher.Input
			placeholder="What can I help you with?"
			class="min-h-[100px] h-fit w-full text-[24px]"
			bind:query={searchQuery}
			bind:editorRef
			onkeydown={handleKeydown}
		/>
	</Launcher.Root>
</div>
