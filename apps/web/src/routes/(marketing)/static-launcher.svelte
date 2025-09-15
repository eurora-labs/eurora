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
	import * as Launcher from '@eurora/prosemirror-view/launcher';
	import { processQuery, clearQuery } from '@eurora/prosemirror-core/util';

	import { Chat } from '@eurora/ui/custom-components/chat/index';
	import * as MessageComponent from '@eurora/ui/custom-components/message/index';
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

	onMount(() => {
		console.log('EditorRef:', launcherInputRef);
	});

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
	<Launcher.Root class="h-[100px]">
		<Launcher.Input
			{placeholder}
			bind:query={searchQuery}
			bind:editorRef={launcherInputRef}
			onkeydown={handleKeydown}
			class="text-black/80"
			disabled={placeholder.length > 0}
		/>
	</Launcher.Root>
	<Chat class="w-full min-h-[265px]">
		{#each messages as message}
			<MessageComponent.Root
				variant={message.role === 'user' ? 'default' : 'assistant'}
				finishRendering={() => {}}
			>
				<MessageComponent.Content>
					{message.content}
				</MessageComponent.Content>
			</MessageComponent.Root>
		{/each}
	</Chat>
</div>
