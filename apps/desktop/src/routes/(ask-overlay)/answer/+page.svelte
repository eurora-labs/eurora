<script lang="ts">
	import { commands } from '$lib/bindings/specta.bindings.js';
	import { TIMELINE_SERVICE } from '$lib/services/timeline-service.svelte.js';
	import { MessageList } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as PromptInput from '@eurora/ui/components/ai-elements/prompt-input/index';
	import { getCurrentWindow } from '@tauri-apps/api/window';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { onMount, tick } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { PromptInputMessage } from '@eurora/ui/components/ai-elements/prompt-input/index';

	const chatService = inject(CHAT_SERVICE);
	const timelineService = inject(TIMELINE_SERVICE);
	const latestApp = $derived(timelineService.latest);

	const activeThreadId = $derived(chatService.activeThreadId);

	let barEl: HTMLDivElement | undefined = $state();

	function focusInput() {
		const textarea = barEl?.querySelector('textarea');
		textarea?.focus();
	}

	onMount(() => {
		const win = getCurrentWindow();
		const initialPrompt = readPromptFromUrl();

		if (initialPrompt) {
			// Start a fresh thread so an in-flight main-window conversation
			// can't be confused with the overlay's question. `sendMessage`
			// will create the thread itself when `activeThreadId` is
			// unset.
			chatService.activeThreadId = undefined;
			chatService.sendMessage(initialPrompt, []).catch((err) => toast.error(String(err)));
		} else {
			// Empty state — focus the input so the user can type
			// immediately. Run after a tick so the textarea is mounted.
			tick().then(focusInput);
		}

		function onKeyDown(event: KeyboardEvent) {
			if (event.key === 'Escape') {
				event.preventDefault();
				closeAnswer();
			}
		}
		document.addEventListener('keydown', onKeyDown);

		// Cancel any in-flight stream the window owns when the OS-level
		// window-close fires. `onMount`'s cleanup runs *after* the
		// webview is already being torn down, which is too late to
		// signal the backend; the close-requested hook runs *before*
		// teardown, which is the correct hand-off point.
		const unlistenClose = win.onCloseRequested(() => {
			if (chatService.activeThreadId) {
				commands.chatCancelQuery(chatService.activeThreadId).catch(() => {});
			}
		});

		return () => {
			document.removeEventListener('keydown', onKeyDown);
			unlistenClose.then((un) => un());
		};
	});

	function readPromptFromUrl(): string | null {
		const params = new URLSearchParams(window.location.search);
		const q = params.get('q');
		return q && q.trim() ? q : null;
	}

	function closeAnswer() {
		getCurrentWindow()
			.close()
			.catch((err) => toast.error(String(err)));
	}

	function handleSubmit(message: PromptInputMessage) {
		const text = message.text.trim();
		if (!text) return;
		chatService.sendMessage(text, []).catch((err) => toast.error(String(err)));
	}

	function handleCopy(content: string) {
		writeText(content).catch((err) => toast.error(`Failed to copy: ${err}`));
	}

	function handleEdit(messageId: string, newText: string) {
		chatService.editMessage(messageId, newText).catch((err) => toast.error(String(err)));
	}

	function handleRegenerate(messageId: string) {
		chatService.regenerateAi(messageId).catch((err) => toast.error(String(err)));
	}
</script>

{#snippet emptyAnswer()}
	<div class="answer-empty">
		<p>Ask Eurora a question to get started.</p>
	</div>
{/snippet}

<div class="answer-pane">
	<div class="answer-bar" data-tauri-drag-region bind:this={barEl}>
		<div class="answer-icon" data-tauri-drag-region>
			{#if latestApp?.iconBase64}
				<img src={latestApp.iconBase64} alt={latestApp.name} />
			{:else}
				<span class="answer-icon-placeholder"></span>
			{/if}
		</div>
		<div class="answer-input">
			<PromptInput.Root onSubmit={handleSubmit}>
				<PromptInput.Body>
					<PromptInput.Textarea
						placeholder={activeThreadId ? 'Ask a follow-up…' : 'Ask Eurora…'}
					/>
				</PromptInput.Body>
			</PromptInput.Root>
		</div>
	</div>
	<div class="answer-stream">
		<MessageList
			emptyState={emptyAnswer}
			onCopy={handleCopy}
			onEdit={handleEdit}
			onRegenerate={handleRegenerate}
		/>
	</div>
</div>

<style>
	.answer-pane {
		display: grid;
		grid-template-rows: auto 1fr;
		height: 100%;
		min-height: 0;
	}
	.answer-bar {
		display: grid;
		grid-template-columns: auto 1fr;
		align-items: center;
		padding: 12px 16px;
		gap: 12px;
		border-bottom: 1px solid color-mix(in oklab, var(--foreground) 8%, transparent);
	}
	.answer-icon {
		display: flex;
		flex: 0 0 auto;
		align-items: center;
		justify-content: center;
		width: 48px;
		height: 48px;
		border-radius: 10px;
		background: color-mix(in oklab, var(--foreground) 6%, transparent);
	}
	.answer-icon img {
		width: 36px;
		height: 36px;
		object-fit: contain;
		image-rendering: -webkit-optimize-contrast;
	}
	.answer-icon-placeholder {
		width: 36px;
		height: 36px;
		border-radius: 8px;
		background: color-mix(in oklab, var(--foreground) 10%, transparent);
	}
	.answer-input {
		flex: 1 1 auto;
		min-width: 0;
	}
	.answer-input :global(.cursor-text) {
		padding: 0;
	}
	.answer-stream {
		display: flex;
		flex-direction: column;
		min-height: 0;
		overflow: hidden;
	}
	.answer-empty {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		padding: 24px;
		color: color-mix(in oklab, var(--foreground) 60%, transparent);
		text-align: center;
	}
</style>
