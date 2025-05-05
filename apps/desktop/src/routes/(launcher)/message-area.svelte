<script lang="ts">
	import { ScrollArea, Card, SystemChatMessage, UserChatMessage } from '@eurora/ui';
	// import SystemChatMessage from '$lib/components/SystemChatMessage.svelte';
	// import UserChatMessage from '$lib/components/UserChatMessage.svelte';

	import { ProtoChatMessage } from '@eurora/proto/questions_service';
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
