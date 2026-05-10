<script lang="ts">
	import { DEFAULT_MODELS } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as ModelSelector from '@eurora/ui/components/ai-elements/model-selector/index';
	import * as PromptInput from '@eurora/ui/components/ai-elements/prompt-input/index';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import GlobeIcon from '@lucide/svelte/icons/globe';

	const chatService = inject(CHAT_SERVICE);

	const selectedModel = $derived(
		DEFAULT_MODELS.find((m) => m.id === chatService.selectedModelId) ?? DEFAULT_MODELS[0],
	);
</script>

<PromptInput.Button size="sm" variant="ghost">
	<ModelSelector.Logo provider={selectedModel.provider} />
	<ModelSelector.Name>{selectedModel.name}</ModelSelector.Name>
</PromptInput.Button>
<PromptInput.Button
	size="sm"
	variant={chatService.searchEnabled ? 'default' : 'ghost'}
	aria-pressed={chatService.searchEnabled}
	onclick={() => (chatService.searchEnabled = !chatService.searchEnabled)}
>
	<GlobeIcon size={16} />
	<span>Search</span>
</PromptInput.Button>
<PromptInput.Button
	size="sm"
	variant={chatService.thinkingEnabled ? 'default' : 'ghost'}
	aria-pressed={chatService.thinkingEnabled}
	onclick={() => (chatService.thinkingEnabled = !chatService.thinkingEnabled)}
>
	<BrainIcon size={16} />
	<span>Thinking</span>
</PromptInput.Button>
