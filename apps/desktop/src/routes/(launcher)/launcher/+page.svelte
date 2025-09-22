<script lang="ts">
	import 'katex/dist/katex.min.css';
	import Katex from '$lib/components/katex.svelte';
	import { listen } from '@tauri-apps/api/event';
	import { scaleFactor } from './scaleFactor.svelte.js';
	import {
		createTauRPCProxy,
		type ResponseChunk,
		type Query,
		type Message,
		type Conversation,
		type ContextChip,
		type LauncherInfo,
	} from '$lib/bindings/bindings.js';

	import * as MessageComponent from '@eurora/ui/custom-components/message/index';

	import { onMount } from 'svelte';
	import { Chat } from '@eurora/ui/custom-components/chat/index';
	import { executeCommand } from '$lib/commands.js';
	import { processQuery, clearQuery, type QueryAssets } from '@eurora/prosemirror-core/util';

	// Import the Launcher component
	import * as Launcher from '@eurora/prosemirror-view/launcher';
	import {
		Editor as ProsemirrorEditor,
		type SveltePMExtension,
	} from '@eurora/prosemirror-core/index';
	// Import the extension factory instead of individual extensions
	import { extensionFactory, registerCoreExtensions } from '@eurora/prosemirror-factory/index';
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
			// extensionFactory.getExtension('9370B14D-B61C-4CE2-BDE7-B18684E8731A'),
			extensionFactory.getExtension('7c7b59bb-d44d-431a-9f4d-64240172e092'),
			extensionFactory.getExtension('309f0906-d48c-4439-9751-7bcf915cdfc5'),
			extensionFactory.getExtension('2c434895-d32c-485f-8525-c4394863b83a'),
		] as SveltePMExtension[],
	});
	let backdropCustom2Ref = $state<HTMLDivElement | null>(null);
	const messages = $state<Message[]>([]);

	let conversation = $state<Conversation | null>(null);

	let backgroundImage = $state<string | null>(null);
	let currentMonitorId = $state<string>('');
	let launcherInfo = $state<LauncherInfo | null>(null);
	let chatRef = $state<Chat | null>(null);

	// Listen for launcher closed event to clear messages and reset conversation
	taurpc.window.launcher_closed.on(() => {
		// Clear messages array
		messages.splice(0, messages.length);
		// Reset current conversation ID to null to default to NEW on next interaction
		conversation = null;
		console.log('Launcher closed: cleared messages and reset conversation');
	});

	taurpc.window.launcher_opened.on(async (info) => {
		await isPromptKitServiceAvailable();
		if (editorRef) {
			clearQuery(editorRef);
		}
		// Reload activities when launcher is opened
		loadActivities();

		// Store the launcher information from the event payload
		launcherInfo = info;
		currentMonitorId = launcherInfo?.monitor_id || '';
		console.log('Launcher opened: refreshed activities, launcher info:', launcherInfo);

		backgroundImage = info.background_image;
		if (!backgroundImage) {
			return;
		}

		const scale = scaleFactor.value;
		console.log('Launcher opened: scale:', scale);
		const img = new Image();
		img.onload = () => {
			if (backdropCustom2Ref && launcherInfo) {
				// For the initial relative image, we can use cover since it's already cropped to the launcher area
				const coverWidth = img.width / scale;
				const coverHeight = img.height / scale;

				backdropCustom2Ref.style.backgroundImage = `url('${backgroundImage}')`;
				// backdropCustom2Ref.style.backgroundSize = `${Math.ceil(coverWidth)}px ${Math.ceil(coverHeight)}px`;
				backdropCustom2Ref.style.backgroundPosition = '0px 0px';
				backdropCustom2Ref.style.backgroundSize = 'cover';
				// backdropCustom2Ref.style.backgroundPosition = 'center';
				backdropCustom2Ref.style.backgroundRepeat = 'no-repeat';
			}
		};
		img.src = backgroundImage;
	});

	taurpc.window.background_image_changed.on(async (fullImageB64) => {
		// Replace the small relative background image with full monitor image while preserving the coordinates
		backgroundImage = fullImageB64;
		const scale = scaleFactor.value;

		// Preload the image to avoid white flash during switch
		const img = new Image();
		img.onload = () => {
			console.log('Image size', img.width, img.height);
			// Only switch once the image is fully loaded
			if (backdropCustom2Ref && launcherInfo) {
				backdropCustom2Ref.style.backgroundImage = `url('${fullImageB64}')`;
				backdropCustom2Ref.style.backgroundSize = `${img.width / scale}px ${img.height / scale}px`;
				backdropCustom2Ref.style.backgroundPosition = `${-launcherInfo.capture_x / scale}px ${-launcherInfo.capture_y / scale}px`;
				backdropCustom2Ref.style.backgroundRepeat = 'no-repeat';
			}
		};
		img.src = fullImageB64;
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

	// Set up global keydown event listener for Escape key
	function handleEscapeKey(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			// When Escape is pressed, set conversation ID to NEW and clear messages
			conversation = null;
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
				messages.push({ role: 'user', content: query.text });
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
			messages.push({ role: 'assistant', content: '' });
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

			// Note: Conversation management is not yet available in TauRPC,
			// so we skip the conversation refresh for now
		} catch (error) {
			console.error('Failed to get answer:', error);
			messages.push({
				role: 'system',
				content: 'Error: Failed to get response from server' + error,
			});
		}
	}

	function triggerResizing(height: number) {
		console.log('resized to ', height);
		taurpc.window.resize_launcher_window(height, scaleFactor.value).then(() => {
			console.log('resized to ', height);
		});
	}
</script>

<div class="backdrop-custom relative overflow-hidden">
	{#if promptKitServiceAvailable}
		<Launcher.Root class="h-fit rounded-lg border-none shadow-none flex flex-col p-0 m-0">
			<Launcher.Input
				placeholder="What can I help you with?"
				bind:query={searchQuery}
				bind:editorRef
				onheightchange={triggerResizing}
				onkeydown={handleKeydown}
				class="min-h-[100px] h-fit w-full text-[40px]"
			/>
		</Launcher.Root>

		{#if messages.length > 0}
			<Chat bind:this={chatRef} class="w-full max-h-[calc(100vh-100px)] flex flex-col gap-4">
				{#each messages as message}
					{#if typeof message.content === 'string'}
						{#if message.content.length > 0}
							<MessageComponent.Root
								variant={message.role === 'user' ? 'default' : 'assistant'}
								finishRendering={() => {}}
							>
								<MessageComponent.Content>
									<Katex math={message.content} finishRendering={() => {}} />
								</MessageComponent.Content>
							</MessageComponent.Root>
						{/if}
					{/if}
				{/each}
			</Chat>
		{/if}
	{:else}
		<div class="flex justify-center items-center h-full flex-col gap-4">
			<h1 class="text-2xl font-bold">Eurora is not initialized</h1>
			<Button onclick={openMainWindow}>Initialize Now</Button>
		</div>
	{/if}
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
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background-color: rgba(255, 255, 255, 0.2);
	}
	:global(body.linux-app .backdrop-custom) {
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background-color: transparent;
	}

	:global(body.linux-app .backdrop-custom-2) {
		backdrop-filter: none;
		-webkit-backdrop-filter: none;
		background-color: transparent;
	}

	:global(body.windows-app .blur-bright) {
		display: none;
	}

	:global(body.mac-app .blur-bright) {
		display: none;
	}
</style>
