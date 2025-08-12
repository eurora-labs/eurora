<script lang="ts">
	import 'katex/dist/katex.min.css';
	import Katex from '$lib/components/katex.svelte';
	import { listen } from '@tauri-apps/api/event';

	import * as Message from '@eurora/ui/custom-components/message/index';
	import {
		ProtoChatMessageSchema,
		type ProtoChatMessage,
	} from '@eurora/shared/proto/questions_service_pb.js';
	import { onMount } from 'svelte';
	import { Chat } from '@eurora/ui/custom-components/chat/index';
	import { executeCommand } from '$lib/commands.js';
	import { processQuery, clearQuery, type QueryAssets } from '@eurora/prosemirror-core/util';
	import {
		createTauRPCProxy,
		type ResponseChunk,
		type Query,
		type ContextChip,
	} from '$lib/bindings/bindings.js';
	import { create } from '@eurora/shared/util/grpc';

	// Import the Launcher component
	import * as Launcher from '@eurora/ui/custom-components/launcher/index';
	import {
		Editor as ProsemirrorEditor,
		type SveltePMExtension,
	} from '@eurora/prosemirror-core/index';
	// Import the extension factory instead of individual extensions
	import { extensionFactory, registerCoreExtensions } from '$lib/prosemirror/index.js';
	import Button from '@eurora/ui/components/button/button.svelte';

	// Create TauRPC proxy
	const taurpc = createTauRPCProxy();
	// Define a type for Conversation based on what we know from main.rs

	let editorRef: ProsemirrorEditor | undefined = $state();
	let promptKitServiceAvailable = $state(false);
	registerCoreExtensions();
	// Query object for the Launcher.Input component
	let searchQuery = $state({
		text: '',
		extensions: [
			extensionFactory.getExtension('9370B14D-B61C-4CE2-BDE7-B18684E8731A'),
			extensionFactory.getExtension('7c7b59bb-d44d-431a-9f4d-64240172e092'),
			extensionFactory.getExtension('309f0906-d48c-4439-9751-7bcf915cdfc5'),
			extensionFactory.getExtension('2c434895-d32c-485f-8525-c4394863b83a'),
		] as SveltePMExtension[],
	});
	let backdropCustom2Ref = $state<HTMLDivElement | null>(null);
	const messages = $state<ProtoChatMessage[]>([]);
	let currentConversationId = $state<string | null>(null);
	let backgroundImage = $state<string | null>(null);
	let currentMonitorId = $state<string>('');
	let launcherInfo = $state<{
		monitor_id: string;
		launcher_x: number;
		launcher_y: number;
		launcher_width: number;
		launcher_height: number;
		monitor_width: number;
		monitor_height: number;
	} | null>(null);
	let chatRef = $state<Chat | null>(null);

	// Listen for launcher closed event to clear messages and reset conversation
	listen('launcher_closed', () => {
		// Clear messages array
		messages.splice(0, messages.length);
		// Reset current conversation ID to null to default to NEW on next interaction
		currentConversationId = null;
		console.log('Launcher closed: cleared messages and reset conversation');
	});

	// Listen for launcher opened event to refresh activities
	listen<any>('launcher_opened', async (event) => {
		await isPromptKitServiceAvailable();
		if (editorRef) {
			clearQuery(editorRef);
		}
		// Reload activities when launcher is opened
		loadActivities();

		// Store the launcher information from the event payload
		launcherInfo = event.payload;
		currentMonitorId = launcherInfo?.monitor_id || '';
		console.log('Launcher opened: refreshed activities, launcher info:', launcherInfo);
	});

	async function isPromptKitServiceAvailable() {
		try {
			const serviceName = await taurpc.prompt.get_service_name();
			console.log('get_service_name', serviceName);
			return serviceName.length > 0;
		} catch (e) {
			console.error('get_service_name failed', e);
			return false;
		}
	}

	async function openMainWindow() {
		try {
			await taurpc.window.open_main_window();
		} catch (e) {
			console.error('open_main_window failed', e);
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

	// Listen for background image event
	listen<string>('background_image', (event) => {
		backgroundImage = event.payload;

		if (backdropCustom2Ref) {
			backdropCustom2Ref.style.backgroundImage = `url('${event.payload}')`;
			backdropCustom2Ref.style.backgroundSize = 'cover';
			backdropCustom2Ref.style.backgroundPosition = 'center';
			backdropCustom2Ref.style.backgroundRepeat = 'no-repeat';
		}
	});

	// Set up global keydown event listener for Escape key
	function handleEscapeKey(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			// When Escape is pressed, set conversation ID to NEW and clear messages
			currentConversationId = 'NEW';
			messages.splice(0, messages.length);
			console.log('Escape pressed: cleared messages and set conversation to NEW');

			// Clear input field if there's any text
			searchQuery.text = '';
		}
	}

	// Add global keydown event listener when component is mounted
	onMount(() => {
		isPromptKitServiceAvailable().then((available) => {
			promptKitServiceAvailable = available;
		});
		document.addEventListener('keydown', handleEscapeKey);

		let unlisten: any;
		taurpc.prompt.prompt_service_change
			.on((name) => {
				promptKitServiceAvailable = name ? name.length > 0 : false;
			})
			.then((unlistenFn) => {
				unlisten = unlistenFn;
			});

		// Clean up event listener when component is unmounted
		return () => {
			document.removeEventListener('keydown', handleEscapeKey);
			unlisten?.();
		};
	});

	// Function to load activities from the backend
	async function loadActivities() {
		try {
			// Note: list_activities is not yet available in TauRPC, fallback to invoke for now
			const result: ContextChip[] = await taurpc.context_chip.get();
			if (!editorRef) return;
			result.forEach((command) => {
				executeCommand(editorRef!, command);
			});
			const query = processQuery(editorRef);
			console.log('query', query);

			console.log('state JSON', editorRef.view?.state.toJSON());
		} catch (error) {
			console.error('Failed to load activities:', error);
		}
	}

	async function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			// await taurpc.window.resize_launcher_window(100, 1.0);

			try {
				const query = processQuery(editorRef!);
				messages.push(
					create(ProtoChatMessageSchema, { role: 'user', content: query.text }),
				);
				console.log('query', query);
				searchQuery.text = '';
				clearQuery(editorRef!);
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

<div class="backdrop-custom relative h-full overflow-hidden">
	<Launcher.Root
		class="h-fit rounded-lg border-none shadow-none flex flex-col p-0 m-0"
		hidden={!promptKitServiceAvailable}
	>
		<Launcher.Input
			placeholder="What can I help you with?"
			bind:query={searchQuery}
			bind:editorRef
			onkeydown={handleKeydown}
			class="min-h-[100px] h-fit w-full"
		/>
	</Launcher.Root>

	{#if messages.length > 0}
		<Chat bind:this={chatRef} class="w-full max-h-[calc(100vh-100px)] flex flex-col gap-4">
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
	{/if}
	<div
		class="flex justify-center items-center h-full flex-col gap-4"
		hidden={promptKitServiceAvailable}
	>
		<h1 class="text-2xl font-bold">Eurora is not initialized</h1>
		<Button onclick={openMainWindow}>Initialize Now</Button>
	</div>
</div>
<svg
	xmlns="http://www.w3.org/2000/svg"
	style="position:absolute;width:0;height:0"
	class="blur-bright"
>
	<filter id="blur-bright" filterUnits="objectBoundingBox">
		<feGaussianBlur
			in="SourceGraphic"
			stdDeviation="36"
			edgeMode="duplicate"
			result="blur"
			color-interpolation-filters="sRGB"
		/>
		<feFlood
			flood-color="#ffffff"
			flood-opacity="0.4"
			result="white"
			color-interpolation-filters="sRGB"
		/>
		<feComposite in="white" in2="blur" operator="over" color-interpolation-filters="sRGB" />
	</filter>
</svg>

<!-- <div
	class="backdrop-custom-2 fixed top-[0px] left-[0px] h-screen w-screen"
	bind:this={backdropCustom2Ref}
></div> -->

<div
	class="backdrop-custom-2 fixed top-[0px] left-[0px] h-screen w-screen"
	style="filter:url(#blur-bright)"
	bind:this={backdropCustom2Ref}
></div>

<style lang="postcss">
	@reference 'tailwindcss';
	:global(.backdrop-custom) {
		z-index: 2;
		backdrop-filter: blur(36px);
		-webkit-backdrop-filter: blur(36px);
		background-color: rgba(255, 255, 255, 0.2);
	}

	:global(.backdrop-custom-2) {
		z-index: 1;
		width: 100%;
		height: 100%;
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background-color: rgba(255, 255, 255, 0.2);
	}
	:global(body.linux-app .backdrop-custom) {
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background: transparent;
		background-color: transparent;
	}

	:global(body.linux-app .backdrop-custom-2) {
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background: transparent;
		background-color: transparent;
	}

	:global(body.windows-app .blur-bright) {
		display: none;
	}

	:global(body.mac-app .blur-bright) {
		display: none;
	}
</style>
