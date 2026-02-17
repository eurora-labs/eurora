<script lang="ts" module>
	export interface Props {
		class?: string;
	}

	export interface Message {
		role: string;
		content: string;
	}
</script>

<script lang="ts">
	import { processQuery, clearQuery } from '@eurora/prosemirror-core/util';
	import * as Launcher from '@eurora/prosemirror-view/launcher';

	import * as Chat from '@eurora/ui/custom-components/chat/index';
	import { cn } from '@eurora/ui/utils';
	import { onMount } from 'svelte';
	let messages: Message[] = $state([
		{
			role: 'user',
			content: 'What am I doing right now?',
		},
		{
			role: 'system',
			content:
				'You are currently looking at a website called Eurora AI. What would you like to know?',
		},
	]);
	let searchQuery = $state({
		text: '',
		extensions: [],
	});
	let launcherInputRef: any = $state();
	let placeholder = $state('');

	let { class: className } = $props();

	onMount(() => {});

	function handleKeydown(event: KeyboardEvent) {
		if (placeholder.length > 0) {
			event.preventDefault();
			return;
		}

		if (event.key === 'Enter' && !event.shiftKey) {
			placeholder = 'What can I help you with?';
			event.preventDefault();
			const value = processQuery(launcherInputRef);
			messages.push({
				role: 'user',
				content: value.text,
			});
			searchQuery.text = '';
			clearQuery(launcherInputRef);
			messages.push({
				role: 'system',
				content: 'Download Eurora to get started',
			});
			launcherInputRef.view?.focus();
		}
	}
</script>

<div class={cn('', className)}>
	<Launcher.Root class="h-25 py-4">
		<Launcher.Input
			{placeholder}
			bind:query={searchQuery}
			bind:editorRef={launcherInputRef}
			onkeydown={handleKeydown}
			class="text-black/80"
			disabled={placeholder.length > 0}
		/>
	</Launcher.Root>
	<Chat.Root class="w-full min-h-66.25">
		{#each messages as message}
			<Chat.Message
				variant={message.role === 'user' ? 'default' : 'assistant'}
				finishRendering={() => {}}
			>
				<Chat.MessageContent>
					{message.content}
				</Chat.MessageContent>
			</Chat.Message>
		{/each}
	</Chat.Root>
</div>
