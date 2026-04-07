<script lang="ts">
	import { FakeThreadService, makeMessageNode } from '../test-utils/fake-thread-service.js';
	import { ChatPromptInput } from '$lib/index.js';
	import { ChatService, CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { provide } from '@eurora/shared/context';

	const fakeService = new FakeThreadService();
	const chatService = new ChatService(fakeService);
	provide(CHAT_SERVICE, chatService);

	let submittedTexts: string[] = $state([]);
	let lastSubmitted = $state('');

	function handleSubmit(text: string) {
		submittedTexts.push(text);
		lastSubmitted = text;
	}

	if (typeof window !== 'undefined') {
		(window as any).__test = {
			fakeService,
			chatService,
			async seedAndLoad(count: number) {
				fakeService.seed(count);
				await chatService.loadThreads(20, 0);
			},
			async selectThread(threadId: string) {
				chatService.activeThreadId = threadId;
				await chatService.loadMessages(threadId);
			},
			async setActiveThreadWithMessages(threadId: string) {
				fakeService.threads = [
					{
						id: threadId,
						title: 'Test Thread',
						createdAt: new Date().toISOString(),
						updatedAt: new Date().toISOString(),
					},
				];
				fakeService.messagesByThread.set(threadId, [
					makeMessageNode('msg-1', 'human', 'Hello'),
					makeMessageNode('msg-2', 'ai', 'Hi there!'),
				]);
				await chatService.loadThreads(20, 0);
				chatService.activeThreadId = threadId;
				await chatService.loadMessages(threadId);
			},
			async setActiveThreadEmpty(threadId: string) {
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
			},
			setStreamChunks(chunks: any[]) {
				fakeService.streamChunks = chunks;
			},
			simulateStreaming(threadId: string) {
				const entry = chatService.getThreadData(threadId);
				if (entry) entry.streamingMessageId = 'fake-streaming-id';
			},
			stopStreaming(threadId: string) {
				const entry = chatService.getThreadData(threadId);
				if (entry) entry.streamingMessageId = null;
			},
			getSubmittedTexts() {
				return submittedTexts;
			},
		};
	}
</script>

<div class="flex h-screen flex-col" data-testid="prompt-container">
	<div class="flex-1"></div>
	<ChatPromptInput
		suggestions={['Tell me a joke', 'Write a poem', 'Explain quantum physics']}
		placeholder="Ask me anything..."
		onSubmit={handleSubmit}
	/>
</div>

<div data-testid="debug-panel" class="fixed top-0 right-0 p-2 text-xs bg-black text-white">
	<span data-testid="last-submitted">{lastSubmitted}</span>
	<span data-testid="submit-count">{submittedTexts.length}</span>
	<span data-testid="active-thread">{chatService.activeThreadId ?? ''}</span>
	<span data-testid="streaming">{!!chatService.activeThread?.streamingMessageId}</span>
</div>
