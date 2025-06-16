<script lang="ts">
	import ConversationItem from './conversation-item.svelte';
	import SendHorizontal from '@lucide/svelte/icons/send-horizontal';
	import type Message from './message.js';
	import { Button } from '@eurora/ui/components/button/index';
	import { Textarea } from '@eurora/ui/components/textarea/index';

	let messages: Message[] = $state([]);
	let inputField: HTMLTextAreaElement;
	let scrollView: any;
	let scrollContainer;

	// export let onSendMessage: (message: string) => Promise<void>;
	const { onSendMessage } = $props();

	function scrollIntoView() {
		if (scrollView) {
			const messages = scrollView.querySelector('#conversation')?.children;
			if (messages && messages.length > 1) {
				const previousMessage = messages[messages.length - 1];
				scrollView.scrollTo({
					top: previousMessage.offsetTop - 200,
					behavior: 'smooth',
				});
			}
		}
	}

	const sendMessage = async () => {
		const inputValue = inputField.value;
		if (!inputValue) return;
		inputField.value = '';
		messages.push({ role: 'user', content: inputValue });
		scrollIntoView();

		await onSendMessage(inputValue);
	};

	function handleKeydown(e: KeyboardEvent) {
		e.stopPropagation();
		e.stopImmediatePropagation();

		if (e.key === 'Enter') {
			e.preventDefault();
			sendMessage();
		}
	}

	function handleKeyup(e: KeyboardEvent) {
		e.stopPropagation();
		e.stopImmediatePropagation();
	}

	function handleKeypress(e: KeyboardEvent) {
		e.stopPropagation();
		e.stopImmediatePropagation();
	}

	export function addMessage(message: Message) {
		messages.push(message);
		scrollIntoView();
	}

	export function clearMessages() {
		messages = [];
	}
</script>

<div class="flex h-full flex-col">
	<div bind:this={scrollView} class="flex-1 overflow-y-auto">
		<div
			bind:this={scrollContainer}
			class="mx-auto flex w-full max-w-3xl flex-col px-4 text-lg"
			id="conversation"
		>
			{#each messages as message}
				<ConversationItem
					finishRendering={() => {
						scrollIntoView();
					}}
					isAgent={message.role === 'system'}
					bind:text={message.content}
					class={message.role === 'user' ? 'ml-auto' : 'mr-auto'}
				/>
			{/each}
		</div>
	</div>
	<div class="bg-background w-full">
		<div class="mx-auto w-full max-w-3xl">
			<div class="p-4">
				<div class="relative rounded-lg bg-white shadow-lg ring-1 ring-black/5">
					<Textarea
						bind:this={inputField as any}
						onkeydown={handleKeydown}
						onkeypress={handleKeypress}
						onkeyup={handleKeyup}
						class="focus:ring-primary/50 min-h-[6rem] resize-none rounded-lg border-0 bg-white pr-12 text-lg shadow-none focus:ring-2"
						placeholder="Type your message..."
					/>
					<Button
						onclick={sendMessage}
						class="absolute right-2 bottom-2 shadow-sm transition-shadow hover:shadow-md"
						size="icon"
					>
						<SendHorizontal />
					</Button>
				</div>
			</div>
		</div>
	</div>
</div>
