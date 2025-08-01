<script lang="ts">
	import { onMount } from 'svelte';
	import {
		createTauRPCProxy,
		type ResponseChunk,
		type Query,
		type ContextChip,
	} from '$lib/bindings/bindings.js';

	import { processQuery, clearQuery, type QueryAssets } from '@eurora/prosemirror-core/util';
	import { goto } from '$app/navigation';
	import { create } from '@eurora/shared/util/grpc';
	import * as Launcher from '@eurora/ui/custom-components/launcher/index';
	import { Chat } from '@eurora/ui/custom-components/chat/index';
	import {
		ProtoChatMessageSchema,
		type ProtoChatMessage,
	} from '@eurora/shared/proto/questions_service_pb.js';
	import * as Message from '@eurora/ui/custom-components/message/index';
	import Katex from '$lib/components/katex.svelte';
	import { extensionFactory, registerCoreExtensions } from '$lib/prosemirror/index.js';
	import { ScrollArea } from '@eurora/ui/components/scroll-area/index';

	import {
		Editor as ProsemirrorEditor,
		type SveltePMExtension,
	} from '@eurora/prosemirror-core/index';

	const messages = $state<ProtoChatMessage[]>([]);
	let status = $state<'loading' | 'ready'>('loading');

	let editorRef: ProsemirrorEditor | undefined = $state();
	let chatRef = $state<Chat | null>(null);

	registerCoreExtensions();
	let searchQuery = $state({
		text: '',
		extensions: [
			extensionFactory.getExtension('9370B14D-B61C-4CE2-BDE7-B18684E8731A'),
			extensionFactory.getExtension('7c7b59bb-d44d-431a-9f4d-64240172e092'),
			extensionFactory.getExtension('309f0906-d48c-4439-9751-7bcf915cdfc5'),
			extensionFactory.getExtension('2c434895-d32c-485f-8525-c4394863b83a'),
		] as SveltePMExtension[],
	});

	const taurpc = createTauRPCProxy();

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

		// addExampleMessages();
	});

	function handleEscapeKey(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			messages.splice(0, messages.length);
			console.log('Escape pressed: cleared messages and set conversation to NEW');
		}
	}

	function addExampleMessages() {
		messages.push(
			create(ProtoChatMessageSchema, {
				role: 'user',
				content: 'What am I doing right now?',
			}),
		);

		messages.push(
			create(ProtoChatMessageSchema, {
				role: 'system',
				content:
					'You are currently looking at a website called Eurora AI. What would you like to know?',
			}),
		);

		messages.push(
			create(ProtoChatMessageSchema, {
				role: 'user',
				content: 'How do I install it?',
			}),
		);
	}

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
				messages.push(
					create(ProtoChatMessageSchema, { role: 'user', content: query.text }),
				);
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
			messages.push(create(ProtoChatMessageSchema, { role: 'agent', content: '' }));
			const agentMessage = messages.at(-1);

			const onEvent = (response: ResponseChunk) => {
				// Append chunk to the last message
				if (agentMessage) {
					agentMessage.content += response.chunk;
				}

				chatRef?.scrollToBottom();
			};

			// Use TauRPC send_query procedure
			await taurpc.chat.send_query(onEvent, tauRpcQuery);

			// Note: Conversation management is not yet available in TauRPC,
			// so we skip the conversation refresh for now
		} catch (error) {
			console.error('Failed to get answer:', error);
			messages.push(
				create(ProtoChatMessageSchema, {
					role: 'system',
					content: 'Error: Failed to get response from server' + error,
				}),
			);
		}
	}
</script>

<div class="absolute top-1/2 w-full h-fit">
	<div class=" flex flex-col justify-center items-center gap-4 w-full">
		{#if messages.length === 0}
			<h1 class="text-2xl font-bold">Eurora is ready!</h1>
			<div class="flex justify-start">
				Either press the hotkey, the Eurora Logo on the right of your screen, or start
				asking questions here.
			</div>
		{/if}
	</div>
</div>

<div class="w-full h-full flex flex-col justify-end items-center gap-4">
	{#if messages.length > 0}
		<ScrollArea
			class="w-full max-h-[calc(80vh-100px)] px-6 flex flex-col justify-end items-center gap-4"
		>
			<Chat bind:this={chatRef} class="w-full h-full flex flex-col gap-4 overflow-hidden">
				{#each messages as message}
					{#if message.content.length > 0}
						<Message.Root
							variant={message.role === 'user' ? 'default' : 'agent'}
							finishRendering={() => {}}
						>
							<Message.Content>
								<Katex math={message.content} finishRendering={() => {}} />
							</Message.Content>
						</Message.Root>
					{/if}
				{/each}
			</Chat>
		</ScrollArea>
	{/if}
	<Launcher.Root class="h-fit rounded-lg border-none shadow-none flex flex-col p-0 m-0 ">
		<Launcher.Input
			placeholder="What can I help you with?"
			class="min-h-[100px] h-fit w-full "
			bind:query={searchQuery}
			bind:editorRef
			onkeydown={handleKeydown}
		/>
	</Launcher.Root>
</div>
