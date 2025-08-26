<script lang="ts" module>
	export interface Props {
		class?: string;
	}
</script>

<script lang="ts">
	import * as Launcher from '@eurora/ui/custom-components/launcher/index';
	import { processQuery, clearQuery } from '@eurora/prosemirror-core/util';
	import {
		ProtoChatMessageSchema,
		type ProtoChatMessage,
	} from '@eurora/shared/proto/questions_service_pb.js';
	import { Chat } from '@eurora/ui/custom-components/chat/index';
	import { create } from '@eurora/shared/util/grpc';
	import * as Message from '@eurora/ui/custom-components/message/index';
	import { cn } from '@eurora/ui/utils';
	import { onMount } from 'svelte';
	let messages: ProtoChatMessage[] = $state([
		create(ProtoChatMessageSchema, {
			role: 'user',
			content: 'What am I doing right now?',
		}),
		create(ProtoChatMessageSchema, {
			role: 'system',
			content:
				'You are currently looking at a website called Eurora AI. What would you like to know?',
		}),
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
			messages.push(
				create(ProtoChatMessageSchema, {
					role: 'user',
					content: value.text,
				}),
			);
			searchQuery.text = '';
			clearQuery(launcherInputRef);
			messages.push(
				create(ProtoChatMessageSchema, {
					role: 'system',
					content: 'Download Eurora to get started',
				}),
			);
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
			<Message.Root
				variant={message.role === 'user' ? 'default' : 'assistant'}
				finishRendering={() => {}}
			>
				<Message.Content>
					{message.content}
				</Message.Content>
			</Message.Root>
		{/each}
	</Chat>
</div>
