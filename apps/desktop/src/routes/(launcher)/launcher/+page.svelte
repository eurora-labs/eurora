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

	// Define Activity type based on the Rust struct
	type ActivityType = 'Article' | 'Application' | 'Browser' | 'Document' | 'Video' | 'Custom';

	type Activity = {
		name: string;
		icon: string;
		process_name: string;
		start: string; // ISO date string
		end: string | null; // ISO date string or null
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
	const activities = $state<Activity[]>([]);

	// Set up event listener for chat responses
	listen<string>('chat_response', (event) => {
		messages.push({ role: 'system', content: event.payload });
	});

	// Listen for launcher closed event to clear messages and reset conversation
	listen('launcher_closed', () => {
		// Clear messages array
		messages.splice(0, messages.length);
		// Reset current conversation ID to null to default to NEW on next interaction
		currentConversationId = null;
		console.log('Launcher closed: cleared messages and reset conversation');
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
			activities.splice(0, activities.length, ...(result as Activity[]));
			console.log('Loaded activities:', activities);
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
		// if (messagesContainer && messages.length > 0) {
		// 	messagesContainer.scrollTop = messagesContainer.scrollHeight;
		// }
	});

	async function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
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

<div class="flex h-screen flex-col">
	<div class="flex flex-wrap gap-2 p-2">
		{#each activities as activity}
			<Badge
				variant="outline"
				class="flex items-center gap-1"
				title={`${activity.name} - Started: ${new Date(activity.start).toLocaleTimeString()}`}
			>
				{#if activity.icon && activity.icon.length > 0}
					<div class="icon-container mr-1 h-4 w-4">
						<img src={activity.icon} alt="Activity Icon" />
					</div>
				{:else}
					📌
				{/if}
				{activity.name}
				<Button size="icon" variant="ghost"><X /></Button>
			</Badge>
		{/each}
		{#if activities.length === 0}
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

		<MessageArea {messages} />
	{/if}
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
</style>
