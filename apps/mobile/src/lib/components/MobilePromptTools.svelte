<script lang="ts">
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as PromptInput from '@eurora/ui/components/ai-elements/prompt-input/index';
	import * as Drawer from '@eurora/ui/components/drawer/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import PlusIcon from '@lucide/svelte/icons/plus';

	const chatService = inject(CHAT_SERVICE);

	let open = $state(false);
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

<PromptInput.Button aria-label="More options" onclick={() => (open = true)}>
	<PlusIcon size={16} />
</PromptInput.Button>

<Drawer.Root bind:open>
	<Drawer.Content>
		<Drawer.Header>
			<Drawer.Title>Conversation options</Drawer.Title>
			<Drawer.Description class="sr-only">
				Toggle web search and extended thinking for this conversation.
			</Drawer.Description>
		</Drawer.Header>
		<div class="flex flex-col gap-1 px-2 pb-2">
			{@render optionRow('Search the web', GlobeIcon, chatService.searchEnabled, () => {
				chatService.searchEnabled = !chatService.searchEnabled;
			})}
			{@render optionRow('Extended thinking', BrainIcon, chatService.thinkingEnabled, () => {
				chatService.thinkingEnabled = !chatService.thinkingEnabled;
			})}
		</div>
	</Drawer.Content>
</Drawer.Root>
