<script lang="ts">
	import { ScrollArea } from '$lib/components/scroll-area/index.js';
	import type MessageType from './message.js';
	import * as Message from '$lib/custom-components/message/index.js';

	interface Props {
		messages?: MessageType[];
		class?: string;
	}

	let { messages = [], class: className }: Props = $props();
</script>

<ScrollArea class="w-full {className}">
	<div class="space-y-4 p-4">
		{#each messages as message}
			<Message.Root
				variant={message.role === 'user' ? 'default' : 'agent'}
				finishRendering={() => {}}
			>
				<Message.Content>{message.content}</Message.Content>
				{#if message.sources && message.sources.length > 0}
					<Message.Footer>
						{#each message.sources as source}
							<Message.Source>{@html source}</Message.Source>
						{/each}
					</Message.Footer>
				{/if}
			</Message.Root>
		{/each}
	</div>
</ScrollArea>
