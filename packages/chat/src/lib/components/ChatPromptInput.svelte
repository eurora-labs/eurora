<script lang="ts" module>
	interface Props {
		suggestions?: string[];
		placeholder?: string;
		header?: Snippet;
		footer?: Snippet;
		onSubmit: (text: string) => void;
		onStop?: () => void;
	}
</script>

<script lang="ts">
	import { CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as PromptInput from '@eurora/ui/components/ai-elements/prompt-input/index';
	import * as Suggestion from '@eurora/ui/components/ai-elements/suggestion/index';
	import type { PromptInputMessage } from '@eurora/ui/components/ai-elements/prompt-input/index';
	import type { Snippet } from 'svelte';

	let {
		suggestions = [],
		placeholder = 'What can I help you with?',
		header,
		footer,
		onSubmit,
		onStop,
	}: Props = $props();

	const chatService = inject(CHAT_SERVICE);

	const streaming = $derived(chatService.activeThread?.streamingMessageId !== null);
	const showSuggestions = $derived(
		suggestions.length > 0 && chatService.activeThread?.messages.length === 0,
	);

	function handleSubmit(message: PromptInputMessage) {
		const text = message.text.trim();
		if (!text) return;
		onSubmit(text);
	}

	function handleSuggestionClick(suggestion: string) {
		onSubmit(suggestion);
	}
</script>

<div class="grid shrink-0 gap-4">
	{#if showSuggestions}
		<Suggestion.Root class="px-4">
			{#each suggestions as suggestion}
				<Suggestion.Item {suggestion} onclick={handleSuggestionClick} />
			{/each}
		</Suggestion.Root>
	{/if}
	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		class="w-full cursor-text px-4 pb-4"
		onclick={(e) => {
			if (
				e.target === e.currentTarget ||
				!(e.target as HTMLElement).closest('textarea, button, a, input')
			) {
				const textarea = e.currentTarget.querySelector('textarea');
				textarea?.focus();
			}
		}}
	>
		<PromptInput.Root onSubmit={handleSubmit}>
			{#if header}
				<PromptInput.Header>
					{@render header()}
				</PromptInput.Header>
			{/if}
			<PromptInput.Body>
				<PromptInput.Textarea {placeholder} />
			</PromptInput.Body>
			<PromptInput.Footer class="justify-end">
				{#if footer}
					{@render footer()}
				{/if}
				<PromptInput.Submit status={streaming ? 'streaming' : 'ready'} {onStop} />
			</PromptInput.Footer>
		</PromptInput.Root>
	</div>
</div>
