<script lang="ts">
	import 'katex/dist/katex.min.css';
	import { Textarea, ScrollArea, Separator, Button, Badge } from '@eurora/ui';
	import { invoke, Channel } from '@tauri-apps/api/core';
	import { listen } from '@tauri-apps/api/event';
	import { ProtoChatMessage } from '@eurora/proto/questions_service';
	import { onMount } from 'svelte';
	import MessageArea from './message-area.svelte';
	import ApiKeyForm from './api-key-form.svelte';

	import { X } from '@lucide/svelte';

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
		messages: ChatMessage[];
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

	// Listen for key events from the Rust backend
	listen<string>('key_event', (event) => {
		console.log('Received key event:', event.payload);

		// If there's text in the input field, handle the key event
		if (inputRef) {
			// Handle special keys
			if (event.payload === 'Escape') {
				// Clear input field and reset conversation
				inputRef.value = '';
				currentConversationId = 'NEW';
				messages.splice(0, messages.length);
			} else if (
				event.payload === 'Backspace' ||
				event.payload === 'Delete' ||
				event.payload === '\b'
			) {
				// Handle backspace key
				if (inputRef.value.length > 0) {
					inputRef.value = inputRef.value.slice(0, -1);
				}
			} else if (event.payload === 'Enter') {
				// Submit the current input
				const question = inputRef.value;
				if (question.trim().length > 0) {
					inputRef.value = '';
					messages.push({ role: 'user', content: question });
					askQuestion(question);
				}
			} else if (event.payload.length === 1 || event.payload === 'Space') {
				// Handle regular character keys and space
				const char = event.payload === 'Space' ? ' ' : event.payload;
				inputRef.value += char;
			}
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
		console.log('Received background image');
		console.log(backgroundImage);
	});

	// Set up global keydown event listener for Escape key
	function handleEscapeKey(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			// When Escape is pressed, set conversation ID to NEW and clear messages
			currentConversationId = 'NEW';
			messages.splice(0, messages.length);
			console.log('Escape pressed: cleared messages and set conversation to NEW');

			// Clear input field if there's any text
			if (inputRef) {
				inputRef.value = '';
			}
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
			const result: { has_key: boolean } = await invoke('check_api_key_exists');
			hasApiKey = result.has_key;

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

	invoke('resize_launcher_window', { height: 500 }).then(() => {
		console.log('Window resized to 500px');
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
			await invoke('resize_launcher_window', { height: 500 });

			try {
				if (inputRef) {
					const question = inputRef.value;
					inputRef.value = '';
					messages.push({ role: 'user', content: question });
					await askQuestion(question);
					// Responses will come through the event listener
				}
			} catch (error) {
				console.error('Error:', error);
			}
		}
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
			await invoke('switch_conversation', { conversationId: id });
			currentConversationId = id;

			// Clear current messages and load messages from the selected conversation
			messages.splice(0, messages.length);

			// Get the conversation details
			const conversation = await invoke('get_current_conversation');
			if (conversation && (conversation as Conversation).id) {
				console.log('Switched to conversation:', conversation);

				// Load messages from this conversation
				if ((conversation as Conversation).messages) {
					// Convert the conversation messages to ProtoChatMessage format
					const conversationMessages = (conversation as Conversation).messages.map((msg) => ({
						role: msg.role,
						content: msg.content
					}));

					// Update the messages array with the conversation messages
					messages.splice(0, messages.length, ...conversationMessages);
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
		invoke('resize_launcher_window', { height: 500 }).catch((error) => {
			console.error('Failed to resize window:', error);
		});
	}
</script>

<div
	class="relative flex h-screen flex-col"
	style={backgroundImage
		? `background-image: url('${backgroundImage}'); background-size: cover; background-position: center;`
		: ''}
>
	<!-- Semi-transparent overlay to ensure content is visible -->
	<div class="absolute inset-0 bg-black/30 backdrop-blur-[2px]"></div>

	<!-- Content container -->
	<div class="relative z-10 flex h-full flex-col">
		<div class="flex flex-wrap gap-2 p-2">
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
		</div>

		<Button
			onclick={() => {
				loadActivities();
			}}>Reload Activities</Button
		>

		{#if isCheckingApiKey}
			<div class="flex h-full items-center justify-center">
				<p class="text-gray-500">Checking API key...</p>
			</div>
		{:else if !hasApiKey}
			<div class="flex h-full items-center justify-center">
				<ApiKeyForm saved={() => onApiKeySaved()} />
			</div>
		{:else}
			<!-- Fixed header with textarea -->
			<div class="flex-none p-2">
				<Textarea
					bind:ref={inputRef}
					class="h-10 w-full text-[34px] leading-[34px]"
					style="font-size: 34px;"
					onkeydown={handleKeydown}
				/>
			</div>
			<!-- Recent conversations list -->
			{#if conversations.length > 0 && messages.length === 0}
				<ScrollArea class="h-72 w-full overflow-y-hidden rounded-md">
					<div class="p-4">
						{#each conversations.slice(0, 5) as conversation}
							<Button
								variant="link"
								class="mx-auto w-full justify-start overflow-hidden"
								onclick={async () => await switchConversation(conversation.id)}
							>
								{conversation.title}
							</Button>
							<Separator class="my-2" />
						{/each}
					</div>
				</ScrollArea>
			{/if}

			<div class="message-scroll-area flex-grow overflow-auto">
				<MessageArea {messages} />
			</div>
		{/if}
	</div>
</div>

<style>
	:global(html, body) {
		overflow: hidden;
		margin-bottom: 100px;
	}

	:global(main) {
		height: 100vh;
		overflow: hidden;
	}

	.icon-container {
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.activity-icon {
		width: 16px;
		height: 16px;
	}

	/* Styles for the transparent effect */
	:global(.message-scroll-area) {
		background-color: rgba(0, 0, 0, 0.4);
		border-radius: 8px;
		margin: 0 8px 8px 8px;
	}

	:global(.textarea) {
		background-color: rgba(255, 255, 255, 0.8) !important;
		backdrop-filter: blur(4px);
	}

	:global(.badge) {
		background-color: rgba(255, 255, 255, 0.7) !important;
		backdrop-filter: blur(4px);
	}
</style>
