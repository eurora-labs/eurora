<script lang="ts">
	import {
		FakeThreadService,
		makeMessageNode,
		makeReasoningNode,
	} from '../test-utils/fake-thread-service.js';
	import { MessageList } from '$lib/index.js';
	import { ChatService, CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { provide } from '@eurora/shared/context';
	import * as Tooltip from '@eurora/ui/components/tooltip/index';
	import { tick } from 'svelte';
	import type { MessageNode } from '$lib/models/messages/index.js';

	const fakeService = new FakeThreadService();
	const chatService = new ChatService(fakeService);
	provide(CHAT_SERVICE, chatService);

	let copiedContent = $state('');
	let editedMessageId = $state('');
	let editedText = $state('');

	function handleCopy(content: string) {
		copiedContent = content;
	}

	function handleEdit(messageId: string, newText: string) {
		editedMessageId = messageId;
		editedText = newText;
	}

	if (typeof window !== 'undefined') {
		(window as any).__test = {
			fakeService,
			chatService,
			makeMessageNode,
			makeReasoningNode,

			async setupThread(threadId: string, messages: MessageNode[]) {
				fakeService.threads = [
					{
						id: threadId,
						title: 'Test Thread',
						createdAt: new Date().toISOString(),
						updatedAt: new Date().toISOString(),
					},
				];
				fakeService.messagesByThread.set(threadId, messages);
				await chatService.loadThreads(20, 0);
				chatService.activeThreadId = threadId;
				await chatService.loadMessages(threadId);
				await tick();
			},

			async setupEmptyThread(threadId: string) {
				fakeService.threads = [
					{
						id: threadId,
						title: 'Test Thread',
						createdAt: new Date().toISOString(),
						updatedAt: new Date().toISOString(),
					},
				];
				fakeService.messagesByThread.set(threadId, []);
				await chatService.loadThreads(20, 0);
				chatService.activeThreadId = threadId;
				await chatService.loadMessages(threadId);
				await tick();
			},

			async simulateStreaming(threadId: string, messageId: string) {
				const entry = chatService.getThreadData(threadId);
				if (entry) entry.streamingMessageId = messageId;
				await tick();
			},

			async stopStreaming(threadId: string) {
				const entry = chatService.getThreadData(threadId);
				if (entry) entry.streamingMessageId = null;
				await tick();
			},

			async appendStreamChunk(threadId: string, messageId: string, text: string) {
				const entry = chatService.getThreadData(threadId);
				if (!entry) return;
				const node = entry.messages.find((n) => n.message.id === messageId);
				if (!node || node.message.type === 'remove') return;
				const existing = node.message.content.find((b) => b.type === 'text');
				if (existing && existing.type === 'text') {
					existing.text += text;
				} else {
					node.message.content.push({
						type: 'text',
						id: null,
						text,
						annotations: [],
						index: null,
						extras: null,
					});
				}
				entry.messages = [...entry.messages];
				await tick();
			},

			setBranchResults(threadId: string, messages: MessageNode[]) {
				fakeService.branchResults.set(threadId, messages);
			},
		};
	}
</script>

<svelte:head>
	<style>
		html,
		body {
			height: 100%;
			margin: 0;
			overflow: hidden;
		}
	</style>
</svelte:head>

<Tooltip.Provider delayDuration={0}>
	<div
		class="flex flex-col overflow-hidden"
		style="height: 100vh"
		data-testid="message-list-container"
	>
		<MessageList onCopy={handleCopy} onEdit={handleEdit} />
	</div>
</Tooltip.Provider>

<div data-testid="debug-panel" class="fixed top-0 right-0 p-2 text-xs bg-black text-white z-50">
	<span data-testid="copied-content">{copiedContent}</span>
	<span data-testid="edited-message-id">{editedMessageId}</span>
	<span data-testid="edited-text">{editedText}</span>
	<span data-testid="active-thread">{chatService.activeThreadId ?? ''}</span>
	<span data-testid="message-count">{chatService.activeThread?.messages.length ?? 0}</span>
</div>
