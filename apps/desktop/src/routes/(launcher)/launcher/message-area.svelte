<script lang="ts">
	import { ScrollArea } from '@eurora/ui/components/scroll-area/index';
	import { SystemChatMessage } from '@eurora/ui/custom-components/system-chat-message/index';
	import { UserChatMessage } from '@eurora/ui/custom-components/user-chat-message/index';
	// import SystemChatMessage from '$lib/components/SystemChatMessage.svelte';
	// import UserChatMessage from '$lib/components/UserChatMessage.svelte';

	// import SystemChatMessage from '$lib/components/SystemChatMessage.svelte';
	// import UserChatMessage from '$lib/components/UserChatMessage.svelte';
	import type { ProtoChatMessage } from '@eurora/shared/proto/questions_service_pb.js';
	import { Katex } from '@eurora/katex';

	interface Props {
		messages: ProtoChatMessage[];
	}
	const { messages }: Props = $props();
</script>

<ScrollArea class="message-scroll-area h-full w-full rounded-md">
	{#each messages as message}
		{#if message.role === 'user'}
			<UserChatMessage>
				{message.content}
			</UserChatMessage>
		{:else}
			<SystemChatMessage>
				<Katex math={message.content} finishRendering={() => {}} />
			</SystemChatMessage>
		{/if}
	{/each}
</ScrollArea>
