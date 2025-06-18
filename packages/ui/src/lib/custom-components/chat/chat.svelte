<script lang="ts" module>
	export interface ChatProps {
		messages?: any[];
		// messages?: MessageType[];
		class?: string;
	}
</script>

<script lang="ts">
	import { ScrollArea } from '$lib/components/scroll-area/index.js';
	import type MessageType from './message.js';
	import * as Message from '$lib/custom-components/message/index.js';

	let { messages = $bindable<MessageType[]>([]), class: className }: ChatProps = $props();

	let scrollAreaRef = $state<HTMLDivElement>();
</script>

<ScrollArea ref={scrollAreaRef} class="w-full {className}">
	<div class="space-y-4 p-4">
		{#each messages as message}
			<!-- <Message.Root
				variant={message.role === 'user' ? 'default' : 'agent'}
				finishRendering={() => {}}
			> -->
			<Message.Root variant="default" finishRendering={() => {}}>
				<Message.Content>{message.content}</Message.Content>
				{#if message.sources && message.sources.length > 0}
					<Message.Footer>
						<Message.Source>
							{#each message.sources as source}
								{@html source}
							{/each}
						</Message.Source>
					</Message.Footer>
				{/if}
			</Message.Root>
		{/each}
	</div>
</ScrollArea>
