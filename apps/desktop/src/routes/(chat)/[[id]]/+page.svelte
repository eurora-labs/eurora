<script lang="ts">
	import { MessageList } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { toast } from 'svelte-sonner';

	let { data } = $props();

	const chatService = inject(CHAT_SERVICE);

	const threadId = $derived(data.threadId);

	$effect(() => {
		if (threadId) {
			chatService.activeThreadId = threadId;
			chatService.loadMessages(threadId);
		}
	});

	function handleCopy(content: string) {
		writeText(content).catch((e) => toast.error(`Failed to copy: ${e}`));
	}
</script>

<div class="flex h-full flex-col overflow-hidden">
	<MessageList onCopy={handleCopy} />
</div>
