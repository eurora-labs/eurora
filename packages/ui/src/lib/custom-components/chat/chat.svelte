<script lang="ts">
	import { ScrollArea } from '$lib/components/scroll-area/index.js';
	import type MessageType from './message.js';
	import * as Message from '$lib/custom-components/message/index.js';

	let messages: MessageType[] = $state([]);
</script>

<ScrollArea>
	{#each messages as message}
		<Message.Root finishRendering={() => {}}>
			<Message.Content>{message.content}</Message.Content>
		</Message.Root>
	{/each}
</ScrollArea>
