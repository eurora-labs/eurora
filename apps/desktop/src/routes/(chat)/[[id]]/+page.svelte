<script lang="ts">
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { MessageList } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { toast } from 'svelte-sonner';

	let { data } = $props();

	const taurpc = inject(TAURPC_SERVICE);
	const chatService = inject(CHAT_SERVICE);

	const threadId = $derived(data.threadId);
	const threadData = $derived(threadId ? chatService.getThreadData(threadId) : undefined);
	const messages = $derived(threadData?.messages ?? []);
	const loading = $derived(threadData?.loading ?? false);
	const streaming = $derived(threadData?.streaming ?? false);

	$effect(() => {
		if (threadId) {
			chatService.activeThreadId = threadId;
			chatService.loadMessages(threadId);
		}
	});

	function handleCopy(content: string) {
		writeText(content).catch((e) => toast.error(`Failed to copy: ${e}`));
	}

	function handleSwitchBranch(messageId: string, direction: number) {
		if (!threadId) return;
		taurpc.thread
			.switch_branch(threadId, messageId, direction)
			.then((updated) => {
				if (threadData) {
					threadData.messages = updated as unknown as typeof threadData.messages;
				}
			})
			.catch((e) => toast.error(`Failed to switch branch: ${e}`));
	}
</script>

<div class="flex h-full flex-col overflow-hidden">
	<MessageList
		{messages}
		{loading}
		{streaming}
		onCopy={handleCopy}
		onSwitchBranch={handleSwitchBranch}
	/>
</div>
