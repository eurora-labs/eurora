<script lang="ts">
	import type { Snippet } from 'svelte';
	import {
		PromptInputControllerState,
		setPromptInputController,
		setProviderAttachments,
	} from './prompt-input-context.svelte.js';

	let {
		initialInput = '',
		children,
	}: {
		initialInput?: string;
		children?: Snippet;
	} = $props();

	const controller = new PromptInputControllerState(initialInput);
	setPromptInputController(controller);
	setProviderAttachments(controller.attachments);
</script>

{@render children?.()}
