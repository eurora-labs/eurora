<script lang="ts">
	import 'katex/dist/katex.min.css';
	import Katex from '$lib/components/katex.svelte';
	import { invoke, Channel } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';

	import * as Message from '@eurora/ui/custom-components/message/index';
	import {
		ProtoChatMessageSchema,
		type ProtoChatMessage,
	} from '@eurora/shared/proto/questions_service_pb.js';
	import { onMount } from 'svelte';
	import { Chat } from '@eurora/ui/custom-components/chat/index';
	// import MessageArea from './message-area.svelte';
	import ApiKeyForm from './api-key-form.svelte';
	import { executeCommand } from '$lib/commands.js';
	import { X, HardDrive, FileTextIcon } from '@lucide/svelte';
	import { processQuery, clearQuery, type QueryAssets } from '@eurora/prosemirror-core/util';
	import { SiGoogledrive } from '@icons-pack/svelte-simple-icons';
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

	// Create TauRPC proxy
	const taurpc = createTauRPCProxy();
	// Define a type for Conversation based on what we know from main.rs

	type Conversation = {
		id: string;
		title: string;
		created_at: number;
		updated_at: number;
	};

	type DisplayAsset = {
		name: string;
		icon: string;
		// process_name: string;
		// start: string; // ISO date string
		// end: string | null; // ISO date string or null
	};

	let inputRef = $state<HTMLTextAreaElement | null>(null);
	let editorRef: ProsemirrorEditor | undefined = $state();
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
	let transcript = $state<string | null>(null);
	const messages = $state<ProtoChatMessage[]>([]);
	let statusCode = $state<string | null>(null);
	let messagesContainer: HTMLElement;
	const conversations = $state<Conversation[]>([]);
	let hasApiKey = $state(true);
	let isCheckingApiKey = $state(false);
	let currentConversationId = $state<string | null>(null);
	const displayAssets = $state<DisplayAsset[]>([]);
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
		// Reload activities when launcher is opened
		loadActivities();

		// Store the launcher information from the event payload
		launcherInfo = event.payload;
		currentMonitorId = launcherInfo?.monitor_id || '';
		console.log('Launcher opened: refreshed activities, launcher info:', launcherInfo);

		// Capture full monitor after launcher is opened to replace the small background
		// try {
		// 	if (currentMonitorId && launcherInfo) {
		// 		// Capture the full monitor using the monitor name from the event
		// 		const fullMonitorImage = await taurpc.monitor.capture_monitor(currentMonitorId);

		// 		// Replace the background image with the full monitor capture
		// 		// Position it so it appears as if looking through transparent glass
		// 		if (backdropCustom2Ref && fullMonitorImage) {
		// 			// Calculate the position offset to align the background properly
		// 			// The background should be positioned so that the launcher area shows
		// 			// the same content as if it were transparent
		// 			const offsetX = -launcherInfo.launcher_x;
		// 			const offsetY = -launcherInfo.launcher_y;

		// 			backdropCustom2Ref.style.backgroundImage = `url('${fullMonitorImage}')`;
		// 			// backdropCustom2Ref.style.backgroundSize = `${launcherInfo.monitor_width}px ${launcherInfo.monitor_height}px`;
		// 			backdropCustom2Ref.style.backgroundSize = `${launcherInfo.monitor_width}px ${launcherInfo.monitor_height}px`;
		// 			backdropCustom2Ref.style.backgroundPosition = `${offsetX}px ${offsetY}px`;
		// 			backdropCustom2Ref.style.backgroundRepeat = 'no-repeat';
		// 			// backdropCustom2Ref.style.backgroundClip = 'content-box';

		// 			// Update the backgroundImage state
		// 			backgroundImage = fullMonitorImage;

		// 			console.log(
		// 				'Full monitor background captured and positioned for monitor:',
		// 				currentMonitorId,
		// 				'offset:',
		// 				offsetX,
		// 				offsetY,
		// 			);
		// 		}
		// 	}
		// } catch (error) {
		// 	console.error('Failed to capture full monitor background:', error);
		// }
	});

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
		// const scrollY = window.scrollY;
		// window.scrollTo(0, 0);
		// taurpc.window.resize_launcher_window(window.outerHeight + scrollY, 1.0).then(() => {
		// 	window.scrollTo(0, 0);
		// });
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
		document.addEventListener('keydown', handleEscapeKey);

		// Check if API key exists
		// checkApiKey();

		// Clean up event listener when component is unmounted
		return () => {
			document.removeEventListener('keydown', handleEscapeKey);
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

	// Function to check if API key exists
	async function checkApiKey() {
		try {
			const result: boolean = await taurpc.third_party.check_api_key_exists();
			hasApiKey = result;

			// If API key exists, initialize the OpenAI client
			if (hasApiKey) {
				await taurpc.third_party.initialize_openai_client();
			}
		} catch (error) {
			console.error('Failed to check API key:', error);
		} finally {
			isCheckingApiKey = false;
		}
	}

	// Load conversations when component is mounted
	// Note: list_conversations is not yet available in TauRPC, fallback to invoke for now
	invoke('list_conversations')
		.then((result) => {
			conversations.splice(0, conversations.length, ...(result as Conversation[]));
			console.log('Loaded conversations:', conversations);
		})
		.catch((error) => {
			console.error('Failed to load conversations:', error);
		});

	// Use TauRPC for window resize
	// taurpc.window.resize_launcher_window(100, 1.0).then(() => {
	// 	console.log('Window resized to 100px');
	// });

	// Auto-scroll to bottom when new messages arrive
	$effect(() => {});

	async function handleKeydown(event: KeyboardEvent) {
		// We still keep the original keyboard handler for direct keyboard input
		// when typing in the input field
		// event.preventDefault();
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

	async function addVideoExtension() {
		editorRef?.cmd((state, dispatch) => {
			const tr = state.tr;
			const { schema } = state;
			const nodes = schema.nodes;
			tr.insert(
				0,
				nodes['9370B14D-B61C-4CE2-BDE7-B18684E8731A'].createChecked(
					{ id: 'video-1', name: 'Some video with attrs' },
					schema.text(' '),
				),
			);
			dispatch?.(tr);
		});
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
				console.log(`got response chunk: ${response.chunk}`);
			};

			// Use TauRPC send_query procedure
			await taurpc.send_query(onEvent, tauRpcQuery);

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

	// Handle API key saved event
	function onApiKeySaved() {
		hasApiKey = true;
		// Resize the window after API key is saved using TauRPC
		// taurpc.window.resize_launcher_window(100, 1.0).catch((error) => {
		// 	console.error('Failed to resize window:', error);
		// });
	}
</script>

<div class="backdrop-custom relative flex h-full flex-col overflow-hidden">
	{#if isCheckingApiKey}
		<div class="flex h-full items-center justify-center">
			<p class="text-gray-500">Checking API key...</p>
		</div>
	{:else if !hasApiKey}
		<div class="flex h-full items-center justify-center">
			<ApiKeyForm saved={() => onApiKeySaved()} />
		</div>
	{:else}
		<!-- Launcher component -->
		<Launcher.Root class="h-fit rounded-lg border-none shadow-none flex flex-col p-0 m-0 ">
			<Launcher.Input
				placeholder="What can I help you with?"
				bind:query={searchQuery}
				bind:editorRef
				onkeydown={handleKeydown}
				class="h-[100px] w-full"
			/>

			<!-- Recent conversations list -->
			{#if messages.length === 0}
				<Launcher.List class="h-full overflow-y-scroll p-0 m-0 max-h-full">
					<!-- <Launcher.List hidden> -->
					<Launcher.Group heading="Local Files">
						<Launcher.Item onclick={addVideoExtension}>
							<HardDrive />
							<span>Video</span>
						</Launcher.Item>
						<Launcher.Item>
							<FileTextIcon />
							<span>Notes</span>
						</Launcher.Item>
					</Launcher.Group>
					<Launcher.Separator />
					<Launcher.Group heading="Google Drive">
						<Launcher.Item>
							<SiGoogledrive />
							<span>Presentation 1</span>
						</Launcher.Item>
						<Launcher.Item>
							<SiGoogledrive />
							<span>Report card</span>
						</Launcher.Item>
						<Launcher.Item>
							<SiGoogledrive />
							<span>Exercise sheet 3</span>
						</Launcher.Item>
					</Launcher.Group>
				</Launcher.List>
			{/if}
		</Launcher.Root>

		{#if messages.length > 0}
			<Chat class="w-full ">
				{#each messages as message}
					<Message.Root
						variant={message.role === 'user' ? 'default' : 'agent'}
						finishRendering={() => {}}
					>
						<Message.Content>
							<Katex math={message.content} finishRendering={() => {}} />
						</Message.Content>
					</Message.Root>
				{/each}
			</Chat>
		{/if}
	{/if}
</div>

<svg xmlns="http://www.w3.org/2000/svg" style="position:absolute;width:0;height:0">
	<filter id="blur-bright" filterUnits="objectBoundingBox">
		<feGaussianBlur in="SourceGraphic" stdDeviation="36" edgeMode="duplicate" result="blur" />
		<feFlood flood-color="#ffffff" flood-opacity="0.1" result="white" />
		<feComposite in="white" in2="blur" operator="over" />
	</filter>
</svg>

<div
	class="backdrop-custom-2 fixed top-[0px] left-[0px] h-screen w-screen"
	style="filter:url(#blur-bright)"
	bind:this={backdropCustom2Ref}
></div>

<style lang="postcss">
	@reference 'tailwindcss';
	.backdrop-custom {
		z-index: 2;
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background-color: rgba(255, 255, 255, 0.2);
		background: transparent;
	}

	.backdrop-custom-2 {
		z-index: 1;
		width: 100%;
		height: 100%;
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
	}
	:global(body.linux-app .backdrop-custom) {
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background: transparent;
	}
	:global(body.linux-app .backdrop-custom-2) {
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background: transparent;
		background-color: transparent;
	}
</style>
