<script lang="ts">
	import { ScrollArea, Card } from '@eurora/ui';
	import SystemChatMessage from '$lib/components/SystemChatMessage.svelte';
	import UserChatMessage from '$lib/components/UserChatMessage.svelte';

	import { ProtoChatMessage } from '@eurora/proto/questions_service';

	interface Props {
		messages: ProtoChatMessage[];
	}
	const { messages }: Props = $props();
</script>

<ScrollArea class="h-full w-full rounded-md">
	{#each messages as message}
		{#if message.role === 'user'}
			<UserChatMessage>
				{message.content}
			</UserChatMessage>
		{:else}
			<SystemChatMessage text={message.content} />
		{/if}
	{/each}
</ScrollArea>
