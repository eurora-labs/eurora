<script lang="ts">
	import { FakeThreadService } from '../test-utils/fake-thread-service.js';
	import { SidebarThreadsList } from '$lib/index.js';
	import { ChatService, CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { provide } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { Toaster } from 'svelte-sonner';

	const fakeService = new FakeThreadService();
	const chatService = new ChatService(fakeService);
	provide(CHAT_SERVICE, chatService);

	let selectedThreadId = $state('');
	let lastAction = $state('');

	if (typeof window !== 'undefined') {
		(window as any).__test = {
			fakeService,
			chatService,
			async seedAndLoad(count: number) {
				fakeService.seed(count);
				await chatService.loadThreads(20, 0);
			},
			async seedWithoutLoad(count: number) {
				fakeService.seed(count);
				await chatService.loadThreads(20, 0);
			},
			async addThread(title: string) {
				const id = `thread-${Date.now()}`;
				fakeService.threads.unshift({
					id,
					title,
					createdAt: new Date().toISOString(),
					updatedAt: new Date().toISOString(),
				});
				await chatService.loadThreads(20, 0);
			},
			async addUntitledThread() {
				const id = `thread-${Date.now()}`;
				fakeService.threads.unshift({
					id,
					title: null as any,
					createdAt: new Date().toISOString(),
					updatedAt: new Date().toISOString(),
				});
				await chatService.loadThreads(20, 0);
			},
			setDeleteDelay(ms: number) {
				fakeService.deleteDelay = ms;
			},
			setDeleteFailure(shouldFail: boolean) {
				fakeService.shouldFailDelete = shouldFail;
			},
		};
	}
</script>

<Toaster />

<div class="flex h-screen w-80" data-testid="sidebar-container">
	<Sidebar.Provider>
		<Sidebar.Root>
			<Sidebar.Content>
				<Sidebar.Group>
					<SidebarThreadsList
						onThreadSelect={(threadId) => {
							selectedThreadId = threadId;
							lastAction = `selected:${threadId}`;
						}}
					/>
				</Sidebar.Group>
			</Sidebar.Content>
		</Sidebar.Root>
	</Sidebar.Provider>
</div>

<div data-testid="debug-panel" class="fixed bottom-0 right-0 p-2 text-xs bg-black text-white">
	<span data-testid="selected-thread-id">{selectedThreadId}</span>
	<span data-testid="last-action">{lastAction}</span>
	<span data-testid="thread-count">{chatService.threads.length}</span>
	<span data-testid="active-thread-id">{chatService.activeThreadId ?? ''}</span>
</div>
