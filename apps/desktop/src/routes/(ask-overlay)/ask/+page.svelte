<script lang="ts">
	import { commands } from '$lib/bindings/specta.bindings.js';
	import { TIMELINE_SERVICE } from '$lib/services/timeline-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as PromptInput from '@eurora/ui/components/ai-elements/prompt-input/index';
	import { onMount, tick } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { PromptInputMessage } from '@eurora/ui/components/ai-elements/prompt-input/index';

	const timelineService = inject(TIMELINE_SERVICE);
	const latestApp = $derived(timelineService.latest);

	let barEl: HTMLDivElement | undefined = $state();

	function focusInput() {
		// `PromptInput.Textarea` doesn't expose a `ref` prop, so query
		// the underlying textarea after mount and focus it. The scoped
		// container ref keeps this from reaching into unrelated DOM if
		// the overlay ever embeds something else with a textarea.
		const textarea = barEl?.querySelector('textarea');
		textarea?.focus();
	}

	onMount(() => {
		// Focus the input as soon as the layout settles so the user can
		// start typing the instant the bar appears.
		tick().then(focusInput);

		// Esc dismisses the bar. Listening at the document level avoids
		// having to thread an onkeydown through every nested element
		// (the textarea swallows the event otherwise).
		function onKeyDown(event: KeyboardEvent) {
			if (event.key === 'Escape') {
				event.preventDefault();
				commands.askCloseWindow().catch((err) => toast.error(String(err)));
			}
		}
		document.addEventListener('keydown', onKeyDown);

		// No auto-hide on blur: spawning the answer window steals focus
		// from the bar, and the user expects the bar to stay so they
		// can keep asking follow-ups. Dismissal is via Esc or the tray
		// entry toggling state. Both windows are always-on-top.

		return () => {
			document.removeEventListener('keydown', onKeyDown);
		};
	});

	function handleSubmit(message: PromptInputMessage) {
		const text = message.text.trim();
		if (!text) return;
		// The bar stays open after submit so the user can keep asking
		// follow-ups without re-summoning. Both windows are always-on-top;
		// the answer pane is anchored above or below the bar by Rust.
		commands.askOpenAnswerWindow(text).catch((err) => toast.error(String(err)));
	}
</script>

<div class="ask-bar" data-tauri-drag-region bind:this={barEl}>
	<div class="ask-icon" data-tauri-drag-region>
		{#if latestApp?.iconBase64}
			<img src={latestApp.iconBase64} alt={latestApp.name} />
		{:else}
			<span class="ask-icon-placeholder"></span>
		{/if}
	</div>
	<div class="ask-input">
		<PromptInput.Root onSubmit={handleSubmit}>
			<PromptInput.Body>
				<PromptInput.Textarea placeholder="Ask Eurora about what you're doing…" />
			</PromptInput.Body>
		</PromptInput.Root>
	</div>
</div>

<style>
	.ask-bar {
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		height: 100%;
		min-height: 0;
		padding: 12px 16px;
		gap: 12px;
	}
	.ask-icon {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border-radius: 10px;
		background: color-mix(in oklab, var(--foreground) 6%, transparent);
	}
	.ask-icon img {
		width: 36px;
		height: 36px;
		object-fit: contain;
		image-rendering: -webkit-optimize-contrast;
	}
	.ask-icon-placeholder {
		width: 36px;
		height: 36px;
		border-radius: 8px;
		background: color-mix(in oklab, var(--foreground) 10%, transparent);
	}
	.ask-input {
		flex: 1 1 auto;
		min-width: 0;
	}
	.ask-input :global(.cursor-text) {
		padding: 0;
	}
</style>
