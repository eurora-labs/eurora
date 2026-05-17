<script lang="ts">
	import { untrack } from 'svelte';
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

	const controller = untrack(() => new PromptInputControllerState(initialInput));
	setPromptInputController(controller);
	setProviderAttachments(controller.attachments);
</script>

{@render children?.()}
