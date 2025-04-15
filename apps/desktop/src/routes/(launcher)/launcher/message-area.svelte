<script lang="ts">
	import { ScrollArea, Card, UserChatMessage, SystemChatMessage } from '@eurora/ui';
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
