<script lang="ts">
	import 'katex/dist/katex.min.css';
	import { Textarea, ScrollArea, Separator, Button, Badge } from '@eurora/ui';
	import { invoke, Channel } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { ProtoChatMessage } from '@eurora/proto/questions_service';
	import { onMount } from 'svelte';
	import MessageArea from './message-area.svelte';
	import ApiKeyForm from './api-key-form.svelte';

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
			extensionFactory.getExtension('9370B14D-B61C-4CE2-BDE7-B18684E8731A')
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

		// Get video extension ID from the factory
		const VIDEO_EXTENSION_ID = '9370B14D-B61C-4CE2-BDE7-B18684E8731A';

		// editorRef?.cmd((state, dispatch) => {
		// 	const tr = state.tr;
		// 	const { schema } = state;
		// 	const nodes = schema.nodes;
		// 	const { $from: from } = state.selection;

		// 	// Check if video node is available in schema
		// 	if (nodes.video) {
		// 		tr.insert(
		// 			from.pos,
		// 			nodes.video.createChecked({ id: 'video-1', text: 'video' }, schema.text(' '))
		// 		);
		// 		dispatch?.(tr);
		// 	} else {
		// 		console.warn('Video node not found in schema');
		// 	}
		// });
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

		// document.body.style.backgroundImage = `url('${event.payload}')`;
		// document.body.style.backgroundSize = '100%';
		// document.body.style.backgroundPosition = 'center';
		// document.body.style.backgroundRepeat = 'no-repeat';
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

		// Load activities
		loadActivities();

		// Clean up event listener when component is unmounted
		return () => {
			document.removeEventListener('keydown', handleEscapeKey);
		};
	});

	// Function to load activities from the backend
	async function loadActivities() {
		try {
			const result = await invoke('list_activities');
			displayAssets.splice(0, displayAssets.length, ...(result as DisplayAsset[]));
			console.log('Loaded activities:', displayAssets);
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
	$effect(() => {
		// We need to use a different approach for auto-scrolling
		// since we can't directly bind to the ScrollArea component
		// const scrollArea = document.querySelector('.message-scroll-area');
		// if (scrollArea && messages.length > 0) {
		// 	setTimeout(() => {
		// 		scrollArea.scrollTop = scrollArea.scrollHeight;
		// 	}, 100);
		// }
		// On background image change body background:
	});

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
				nodes.video.createChecked(
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

	// Function to switch to a selected conversation
	async function switchConversation(id: string) {
		try {
			// Clear current messages and load messages from the selected conversation
			messages.splice(0, messages.length);
			const [conversation, chat_messages] = (await invoke('get_conversation_with_messages', {
				conversationId: id
			})) as [Conversation, ChatMessage[]];

			if (conversation && conversation.id) {
				console.log('Switched to conversation:', conversation);

				chat_messages.push({
					role: 'user',
					content: 'test sfadf asdf sdiong sfdipgn siodnv psodmv pisdnpin'
				} as any);

				// Load messages from this conversation
				if (chat_messages) {
					// Convert the conversation messages to ProtoChatMessage format

					// Update the messages array with the conversation messages
					// messages.splice(0, messages.length, ...conversationMessages);
					messages.splice(0, messages.length, ...chat_messages);
					console.log('Loaded messages:', messages);
				}
			}
		} catch (error) {
			console.error('Failed to switch conversation:', error);
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
	<!-- <div
	class="relative flex h-screen flex-col"
	style={backgroundImage
		? `background-image: url('${backgroundImage}'); background-size: cover; background-position: center;`
		: ''}
> -->
	<!-- Semi-transparent overlay to ensure content is visible -->
	<!-- <div class="absolute inset-0 bg-black/30 backdrop-blur-[2px]"></div> -->

	<!-- Content container -->
	<div class="relative z-10 flex h-full flex-col">
		<button onclick={addVideoExtension} class="absolute right-2 top-2">Add video</button>
		<!-- <div class="flex flex-wrap gap-2 p-2">
			{#each displayAssets as asset, index}
				<Badge variant="outline" class="flex items-center gap-1" title={`${asset.name}`}>
					{#if asset.icon && asset.icon.length > 0}
						<div class="icon-container mr-1 h-4 w-4">
							<img src={asset.icon} alt="Activity Icon" />
						</div>
					{:else}
						📌
					{/if}
					{asset.name}
					<Button
						size="icon"
						variant="ghost"
						onclick={() => {
							displayAssets.splice(index, 1);
						}}
					>
						<X />
					</Button>
				</Badge>
			{/each}
			{#if displayAssets.length === 0}
				<Badge variant="outline">No recent activities</Badge>
			{/if}
		</div> -->
		<!--
		<Button
			onclick={() => {
				loadActivities();
			}}>Reload Activities</Button
		> -->

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
						<!-- <Launcher.List> -->
						<Launcher.List hidden>
							<Launcher.Group heading="Local Files">
								<Launcher.Item>
									<HardDrive />
									<span>Exercise Sheet 2</span>
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
							<!-- <Launcher.Group heading="Recent Conversations">
								{#each conversations.slice(0, 3) as conversation}
									<Launcher.Item onclick={async () => await switchConversation(conversation.id)}>
										<FileTextIcon />
										<span>{conversation.title}</span>
									</Launcher.Item>
								{/each}
							</Launcher.Group> -->
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

<style>
	.backdrop-custom {
		backdrop-filter: blur(18px);
		-webkit-backdrop-filter: blur(18px);
		background-color: rgba(255, 255, 255, 0.2);
		z-index: 2;
	}

	.backdrop-custom-2 {
		/* filter: blur(18px); */
		/* -webkit-filter: blur(18px); */

		width: 100%;
		height: 100%;

		z-index: 1;

		background-color: rgba(0, 0, 0, 1);
	}
</style>
