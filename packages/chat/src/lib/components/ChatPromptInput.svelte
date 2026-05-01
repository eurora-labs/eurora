<script lang="ts" module>
	import type { Suggestion } from '$lib/models/suggestion.js';
	import type { Snippet } from 'svelte';

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
	import * as Drawer from '@eurora/ui/components/drawer/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import PlusIcon from '@lucide/svelte/icons/plus';
	import type { PromptInputMessage } from '@eurora/ui/components/ai-elements/prompt-input/index';

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

	const models = [{ id: 'glm-5.1', name: 'GLM-5.1: Multimodal', provider: 'zai' }];

	let selectedModelId = $state(models[0].id);
	let searchEnabled = $state(true);
	let thinkingEnabled = $state(true);
	let optionsOpen = $state(false);

	const selectedModel = $derived(models.find((m) => m.id === selectedModelId) ?? models[0]);

	function handleSubmit(message: PromptInputMessage) {
		const text = message.text.trim();
		if (!text) return;
		onSubmit(text);
	}
</script>

{#snippet optionRow(label: string, Icon: typeof GlobeIcon, checked: boolean, onToggle: () => void)}
	<button
		type="button"
		aria-pressed={checked}
		onclick={onToggle}
		class="hover:bg-accent active:bg-accent/80 flex w-full items-center gap-3 rounded-md px-4 py-3 text-left transition-colors"
	>
		<Icon class="text-muted-foreground size-5" />
		<span class="flex-1 text-sm font-medium">{label}</span>
		<Switch {checked} aria-hidden="true" tabindex={-1} class="pointer-events-none" />
	</button>
{/snippet}

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
		class="w-full cursor-text px-4 pb-[max(1rem,env(safe-area-inset-bottom))]"
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
				<PromptInput.Tools class="hidden sm:flex">
					<PromptInput.Button size="sm" variant="ghost">
						<ModelSelector.Logo provider={selectedModel.provider} />
						<ModelSelector.Name>{selectedModel.name}</ModelSelector.Name>
					</PromptInput.Button>
					<PromptInput.Button
						size="sm"
						variant={searchEnabled ? 'default' : 'ghost'}
						aria-pressed={searchEnabled}
						onclick={() => (searchEnabled = !searchEnabled)}
					>
						<GlobeIcon size={16} />
						<span>Search</span>
					</PromptInput.Button>
					<PromptInput.Button
						size="sm"
						variant={thinkingEnabled ? 'default' : 'ghost'}
						aria-pressed={thinkingEnabled}
						onclick={() => (thinkingEnabled = !thinkingEnabled)}
					>
						<BrainIcon size={16} />
						<span>Thinking</span>
					</PromptInput.Button>
				</PromptInput.Tools>
				<PromptInput.Button
					class="sm:hidden"
					aria-label="More options"
					onclick={() => (optionsOpen = true)}
				>
					<PlusIcon size={16} />
				</PromptInput.Button>
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

<Drawer.Root bind:open={optionsOpen}>
	<Drawer.Content>
		<Drawer.Header>
			<Drawer.Title>Conversation options</Drawer.Title>
			<Drawer.Description class="sr-only">
				Toggle web search and extended thinking for this conversation.
			</Drawer.Description>
		</Drawer.Header>
		<div class="flex flex-col gap-1 px-2 pb-2">
			{@render optionRow('Search the web', GlobeIcon, searchEnabled, () => {
				searchEnabled = !searchEnabled;
			})}
			{@render optionRow('Extended thinking', BrainIcon, thinkingEnabled, () => {
				thinkingEnabled = !thinkingEnabled;
			})}
		</div>
	</Drawer.Content>
</Drawer.Root>
