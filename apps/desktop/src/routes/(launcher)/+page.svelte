<script lang="ts">
	import 'katex/dist/katex.min.css';
	import { Textarea, ScrollArea, Separator, Button, Badge } from '@eurora/ui';
	import { invoke, Channel } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { ProtoChatMessage } from '@eurora/proto/questions_service';
	import { onMount } from 'svelte';
	import MessageArea from './message-area.svelte';
	import ApiKeyForm from './api-key-form.svelte';
	import { executeCommand, type PMCommand } from '$lib/commands.js';
	import { X, HardDrive, FileTextIcon } from '@lucide/svelte';

	import { SiGoogledrive } from '@icons-pack/svelte-simple-icons';

	// Import the Launcher component
	import { Launcher } from '@eurora/launcher';
	import { Editor as ProsemirrorEditor, type SveltePMExtension } from '@eurora/prosemirror-core';
	// Import the extension factory instead of individual extensions
	import { extensionFactory, registerCoreExtensions } from '@eurora/prosemirror-factory';
	// Define a type for Conversation based on what we know from main.rs
	type ChatMessage = {
		id: string;
		role: string;
		content: string;
		visible: boolean;
		created_at: number;
		updated_at: number;
	};

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
			extensionFactory.getExtension('7c7b59bb-d44d-431a-9f4d-64240172e092')
		] as SveltePMExtension[]
	});
	let backdropCustom2Ref = $state<HTMLDivElement | null>(null);
	let transcript = $state<string | null>(null);
	const messages = $state<ProtoChatMessage[]>([]);
	let statusCode = $state<string | null>(null);
	let messagesContainer: HTMLElement;
	const conversations = $state<Conversation[]>([]);
	let hasApiKey = $state(false);
	let isCheckingApiKey = $state(true);
	let currentConversationId = $state<string | null>(null);
	const displayAssets = $state<DisplayAsset[]>([]);
	let backgroundImage = $state<string | null>(null);

	// Set up event listener for chat responses
	listen<string>('chat_response', (event) => {
		messages.push({ role: 'system', content: event.payload });
	});

	listen<string>('add_video_context_chip', (event) => {});

	// Listen for key events from the Rust backend
	listen<string>('key_event', (event) => {
		console.log('Received key event:', event.payload);

		// Handle special keys
		if (event.payload === 'Escape') {
			// Clear input field and reset conversation
			searchQuery.text = '';
			currentConversationId = 'NEW';
			messages.splice(0, messages.length);
		} else if (
			event.payload === 'Backspace' ||
			event.payload === 'Delete' ||
			event.payload === '\b'
		) {
			// Handle backspace key
			if (searchQuery.text.length > 0) {
				searchQuery.text = searchQuery.text.slice(0, -1);
			}
		} else if (event.payload === 'Enter') {
			// Submit the current input
			const question = searchQuery.text;
			if (question.trim().length > 0) {
				searchQuery.text = '';
				messages.push({ role: 'user', content: question });
				askQuestion(question);
			}
		} else if (event.payload.length === 1 || event.payload === 'Space') {
			// Handle regular character keys and space
			const char = event.payload === 'Space' ? ' ' : event.payload;
			searchQuery.text += char;
		}
	});

	// Listen for launcher closed event to clear messages and reset conversation
	listen('launcher_closed', () => {
		// Clear messages array
		messages.splice(0, messages.length);
		// Reset current conversation ID to null to default to NEW on next interaction
		currentConversationId = null;
		console.log('Launcher closed: cleared messages and reset conversation');
	});

	// Listen for launcher opened event to refresh activities
	listen('launcher_opened', () => {
		// Reload activities when launcher is opened
		loadActivities();

		console.log('Launcher opened: refreshed activities');
	});

	// Listen for background image event
	listen<string>('background_image', (event) => {
		backgroundImage = event.payload;

		if (backdropCustom2Ref) {
			backdropCustom2Ref.style.backgroundImage = `url('${event.payload}')`;
			backdropCustom2Ref.style.backgroundSize = '150%';
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
		checkApiKey();

		// Clean up event listener when component is unmounted
		return () => {
			document.removeEventListener('keydown', handleEscapeKey);
		};
	});

	// Function to load activities from the backend
	async function loadActivities() {
		try {
			const result: PMCommand[] = await invoke('list_activities');
			if (!editorRef) return;
			result.forEach((command) => {
				executeCommand(editorRef!, command);
			});
			console.log('state JSON', editorRef.view?.state.toJSON());
		} catch (error) {
			console.error('Failed to load activities:', error);
		}
	}

	// Function to check if API key exists
	async function checkApiKey() {
		try {
			const result: boolean = await invoke('check_api_key_exists');
			hasApiKey = result;

			// If API key exists, initialize the OpenAI client
			if (hasApiKey) {
				await invoke('initialize_openai_client');
			}
		} catch (error) {
			console.error('Failed to check API key:', error);
		} finally {
			isCheckingApiKey = false;
		}
	}

	// Load conversations when component is mounted
	invoke('list_conversations')
		.then((result) => {
			conversations.splice(0, conversations.length, ...(result as Conversation[]));
			console.log('Loaded conversations:', conversations);
		})
		.catch((error) => {
			console.error('Failed to load conversations:', error);
		});

	invoke('resize_launcher_window', { height: 100 }).then(() => {
		console.log('Window resized to 100px');
	});

	// Auto-scroll to bottom when new messages arrive
	$effect(() => {});

	async function handleKeydown(event: KeyboardEvent) {
		// We still keep the original keyboard handler for direct keyboard input
		// when typing in the input field
		// event.preventDefault();
		if (event.key === 'Enter' && !event.shiftKey) {
			await invoke('resize_launcher_window', { height: 100 });

			try {
				const question = searchQuery.text;
				searchQuery.text = '';
				messages.push({ role: 'user', content: question });
				await askQuestion(question);
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
					{ id: 'video-1', text: 'Some video with attrs' },
					schema.text('video')
				)
			);
			dispatch?.(tr);
		});
	}
	async function askQuestion(question: string): Promise<void> {
		try {
			type DownloadEvent =
				| {
						event: 'message';
						data: {
							message: string;
						};
				  }
				| {
						event: 'append';
						data: {
							chunk: string;
						};
				  };

			const onEvent = new Channel<DownloadEvent>();
			onEvent.onmessage = (message) => {
				if (message.event == 'message') {
					messages.push({
						role: 'system',
						content: message.data.message
					});
				} else {
					messages.at(-1)!.content += message.data.chunk;
				}
				console.log(`got download event ${message.event}`);
			};

			// Use the current conversation ID if one is selected and we have existing messages,
			// otherwise create a new one
			const conversationId =
				messages.length > 0 && currentConversationId ? currentConversationId : 'NEW';

			if (conversationId === 'NEW') {
				await invoke('ask_video_question', { conversationId, question, channel: onEvent });
			} else {
				await invoke('continue_conversation', { conversationId, question, channel: onEvent });
			}

			// If we created a new conversation, refresh the conversation list
			if (conversationId === 'NEW') {
				invoke('list_conversations')
					.then((result) => {
						conversations.splice(0, conversations.length, ...(result as Conversation[]));
					})
					.catch((error) => {
						console.error('Failed to refresh conversations:', error);
					});
			}
		} catch (error) {
			console.error('Failed to get answer:', error);
			messages.push({
				role: 'system',
				content: 'Error: Failed to get response from server' + error
			});
		}
	}

	// Handle API key saved event
	function onApiKeySaved() {
		hasApiKey = true;
		// Resize the window after API key is saved
		invoke('resize_launcher_window', { height: 100 }).catch((error) => {
			console.error('Failed to resize window:', error);
		});
	}
</script>

<div class="backdrop-custom relative flex h-full flex-col">
	<div class="relative z-10 flex h-full flex-col">
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
			<div class="flex-none p-0">
				<Launcher.Root class="rounded-lg border-none shadow-none">
					<Launcher.Input
						placeholder="Search"
						bind:query={searchQuery}
						bind:editorRef
						onkeydown={handleKeydown}
						class="h-[100px]"
					/>

					<!-- Recent conversations list -->
					{#if messages.length === 0}
						<Launcher.List>
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
			</div>

			<div class="message-scroll-area w-full flex-grow overflow-auto">
				<MessageArea {messages} />
			</div>
		{/if}
	</div>
</div>
<div
	class="backdrop-custom-2 fixed left-[0px] top-[0px] h-screen w-screen"
	bind:this={backdropCustom2Ref}
></div>

<style lang="postcss">
	.backdrop-custom {
		backdrop-filter: blur(18px);
		-webkit-backdrop-filter: blur(18px);
		background-color: rgba(255, 255, 255, 0.2);
		z-index: 2;
	}

	.backdrop-custom-2 {
		width: 100%;
		height: 100%;

		z-index: 1;

		background-color: rgba(0, 0, 0, 1);
	}
	:global(body.linux-app .backdrop-custom) {
		background: transparent;
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
	}
	:global(body.linux-app .backdrop-custom-2) {
		background: transparent;
		background-color: transparent;
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
	}
</style>
