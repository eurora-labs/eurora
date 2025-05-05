<script lang="ts">
	import { AiChat } from '@eurora/ai-chat';
	import { onMount } from 'svelte';
	import { chatMessages, askQuestion, loadChatMessages } from '$lib/chat-store';

	let aiChat = $state<AiChat | null>(null);
	let hasMessages = $derived($chatMessages.length > 0);

	onMount(async () => {
		await loadChatMessages();
	});

	async function handleMessage(message: string) {
		try {
			// This will send the question to the backend and get a response
			await askQuestion(message);
		} catch (error) {
			console.error('Error asking question:', error);
		}
	}
</script>

<div class="flex h-full flex-1 flex-col overflow-hidden">
	<div class="relative h-full flex-1 overflow-auto">
		<div
			class={hasMessages
				? 'flex h-full flex-col justify-between'
				: 'absolute inset-0 flex flex-col items-center justify-center'}
		>
			{#if !hasMessages}
				<h2 class="mb-4 text-center text-2xl font-semibold text-foreground">
					What can I help you with?
				</h2>
			{/if}
			<div class={hasMessages ? 'flex-1 overflow-y-auto' : 'w-full max-w-3xl px-4'}>
				<AiChat bind:this={aiChat} onSendMessage={handleMessage} />
			</div>
		</div>
	</div>
</div>

<style>
	:host {
		display: flex;
		z-index: 9999;
		position: relative;
		height: 100%;
		visibility: hidden;
	}

	.conversation-text {
		user-select: text;
		cursor: text;
	}
</style>
