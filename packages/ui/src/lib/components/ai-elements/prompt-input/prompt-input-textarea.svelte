<script lang="ts">
	import type { HTMLTextareaAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import {
		useOptionalPromptInputController,
		usePromptInputAttachments,
	} from './prompt-input-context.svelte.js';

	let {
		class: className,
		placeholder = 'What would you like to know?',
		oninput = undefined,
		onkeydown = undefined,
		...restProps
	}: HTMLTextareaAttributes & {
		oninput?: (e: Event & { currentTarget: HTMLTextAreaElement }) => void;
		onkeydown?: (e: KeyboardEvent & { currentTarget: HTMLTextAreaElement }) => void;
	} = $props();

	const controller = useOptionalPromptInputController();
	const attachments = usePromptInputAttachments();
	let isComposing = $state(false);

	function handleKeyDown(e: KeyboardEvent & { currentTarget: HTMLTextAreaElement }) {
		onkeydown?.(e);

		if (e.defaultPrevented) return;

		if (e.key === 'Enter') {
			if (isComposing) return;
			if (e.shiftKey) return;
			e.preventDefault();

			const form = e.currentTarget.form;
			const submitButton = form?.querySelector(
				'button[type="submit"]',
			) as HTMLButtonElement | null;
			if (submitButton?.disabled) return;

			form?.requestSubmit();
		}

		if (e.key === 'Backspace' && e.currentTarget.value === '' && attachments.files.length > 0) {
			e.preventDefault();
			const lastAttachment = attachments.files.at(-1);
			if (lastAttachment) {
				attachments.remove(lastAttachment.id);
			}
		}
	}

	function handlePaste(e: ClipboardEvent & { currentTarget: HTMLTextAreaElement }) {
		const items = e.clipboardData?.items;
		if (!items) return;

		const pastedFiles: File[] = [];
		for (const item of items) {
			if (item.kind === 'file') {
				const file = item.getAsFile();
				if (file) pastedFiles.push(file);
			}
		}

		if (pastedFiles.length > 0) {
			e.preventDefault();
			attachments.add(pastedFiles);
		}
	}

	function handleCompositionStart() {
		isComposing = true;
	}

	function handleCompositionEnd() {
		isComposing = false;
	}

	function handleInput(e: Event & { currentTarget: HTMLTextAreaElement }) {
		if (controller) {
			controller.textInput.setInput(e.currentTarget.value);
		}
		oninput?.(e);
	}
</script>

<textarea
	data-slot="prompt-input-textarea"
	class={cn(
		'field-sizing-content max-h-48 min-h-16 w-full resize-none border-none bg-transparent px-3 py-2 outline-none placeholder:text-muted-foreground focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-50',
		className,
	)}
	name="message"
	{placeholder}
	oncompositionstart={handleCompositionStart}
	oncompositionend={handleCompositionEnd}
	onkeydown={handleKeyDown}
	onpaste={handlePaste}
	oninput={handleInput}
	value={controller ? controller.textInput.value : undefined}
	{...restProps}
></textarea>
