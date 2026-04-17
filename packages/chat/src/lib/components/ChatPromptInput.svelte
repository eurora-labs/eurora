<script lang="ts" module>
	interface Props {
		suggestions?: Suggestion[];
		placeholder?: string;
		header?: Snippet;
		footer?: Snippet;
		onSubmit: (text: string) => void;
	}
</script>

<script lang="ts">
	import { CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as ModelSelector from '@eurora/ui/components/ai-elements/model-selector/index';
	import * as PromptInput from '@eurora/ui/components/ai-elements/prompt-input/index';
	import * as SuggestionUI from '@eurora/ui/components/ai-elements/suggestion/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import type { Suggestion } from '$lib/models/suggestion.js';
	import type { PromptInputMessage } from '@eurora/ui/components/ai-elements/prompt-input/index';
	import type { Snippet } from 'svelte';

	let {
		suggestions = [],
		placeholder = 'What can I help you with?',
		header,
		footer,
		onSubmit,
	}: Props = $props();

	const chatService = inject(CHAT_SERVICE);

	const streaming = $derived(!!chatService.activeThread?.streamingMessageId);
	const showSuggestions = $derived(
		suggestions.length > 0 && (chatService.activeThread?.messages.length ?? 0) === 0,
	);

	const models = [{ id: 'glm-5.1', name: 'GLM-5.1', provider: 'zai' }];

	let selectedModelId = $state(models[0].id);
	let modelSelectorOpen = $state(false);

	const selectedModel = $derived(models.find((m) => m.id === selectedModelId) ?? models[0]);

	function handleModelSelect(id: string) {
		selectedModelId = id;
		modelSelectorOpen = false;
	}

	function handleSubmit(message: PromptInputMessage) {
		const text = message.text.trim();
		if (!text) return;
		onSubmit(text);
	}
</script>

<div class="grid shrink-0 gap-4">
	{#if showSuggestions}
		<SuggestionUI.Root class="px-4">
			{#each suggestions as suggestion (suggestion.label)}
				<SuggestionUI.Item suggestion={suggestion.label} onclick={suggestion.onSelect} />
			{/each}
		</SuggestionUI.Root>
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
			<PromptInput.Footer>
				<PromptInput.Tools>
					<ModelSelector.Root bind:open={modelSelectorOpen}>
						<ModelSelector.Trigger>
							<PromptInput.Button size="sm">
								<ModelSelector.Logo provider={selectedModel.provider} />
								<ModelSelector.Name>{selectedModel.name}</ModelSelector.Name>
							</PromptInput.Button>
						</ModelSelector.Trigger>
						<ModelSelector.Content>
							<ModelSelector.Input placeholder="Search models..." />
							<ModelSelector.List>
								<ModelSelector.Empty>No models found.</ModelSelector.Empty>
								<ModelSelector.Group heading="Z.AI">
									{#each models as m (m.id)}
										<ModelSelector.Item
											value={m.id}
											onSelect={() => handleModelSelect(m.id)}
										>
											<ModelSelector.Logo provider={m.provider} />
											<ModelSelector.Name>{m.name}</ModelSelector.Name>
											{#if selectedModelId === m.id}
												<CheckIcon class="ml-auto size-4" />
											{:else}
												<div class="ml-auto size-4"></div>
											{/if}
										</ModelSelector.Item>
									{/each}
								</ModelSelector.Group>
							</ModelSelector.List>
						</ModelSelector.Content>
					</ModelSelector.Root>
				</PromptInput.Tools>
				<div class="flex items-center gap-1">
					{#if footer}
						{@render footer()}
					{/if}
					<PromptInput.Submit
						status={streaming ? 'streaming' : 'ready'}
						onStop={() => chatService.abortController?.abort()}
					/>
				</div>
			</PromptInput.Footer>
		</PromptInput.Root>
	</div>
</div>
