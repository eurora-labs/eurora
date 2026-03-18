<script lang="ts">
	import { goto } from '$app/navigation';
	import type {
		SearchMessageResultView,
		SearchThreadResultView,
	} from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { THREAD_SERVICE } from '$lib/services/thread-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as Command from '@eurora/ui/components/command/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import FileTextIcon from '@lucide/svelte/icons/file-text';
	import MessageSquareIcon from '@lucide/svelte/icons/message-square';
	let { open = $bindable(false) }: { open?: boolean } = $props();

	const taurpc = inject(TAURPC_SERVICE);
	const threadService = inject(THREAD_SERVICE);

	let query = $state('');
	let threadResults: SearchThreadResultView[] = $state([]);
	let messageResults: SearchMessageResultView[] = $state([]);
	let loading = $state(false);
	let debounceTimer: ReturnType<typeof setTimeout> | undefined;

	$effect(() => {
		if (!open) {
			query = '';
			threadResults = [];
			messageResults = [];
		}
	});

	$effect(() => {
		const q = query.trim();
		clearTimeout(debounceTimer);

		if (q.length < 2) {
			threadResults = [];
			messageResults = [];
			return;
		}

		loading = true;
		debounceTimer = setTimeout(async () => {
			try {
				const [threads, messages] = await Promise.all([
					taurpc.thread.search_threads(q, 10, 0),
					taurpc.thread.search_messages(q, 10, 0),
				]);
				threadResults = threads;
				messageResults = messages;
			} catch (e) {
				console.error('[search] failed:', e);
			} finally {
				loading = false;
			}
		}, 300);

		return () => clearTimeout(debounceTimer);
	});

	function selectThread(id: string) {
		open = false;
		threadService.activeThreadId = id;
		goto(`/${id}`);
	}

	function selectMessage(threadId: string) {
		open = false;
		threadService.activeThreadId = threadId;
		goto(`/${threadId}`);
	}

	function sanitizeSnippet(html: string): string {
		return html
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;')
			.replace(/&lt;mark&gt;/g, '<mark>')
			.replace(/&lt;\/mark&gt;/g, '</mark>');
	}
</script>

<Command.Dialog
	bind:open
	title="Search"
	description="Search through chats and messages"
	shouldFilter={false}
>
	<Command.Input
		placeholder="Search chats and messages..."
		bind:value={query}
		class="!border-0 !shadow-none !ring-0 !outline-none"
	/>
	<Command.List>
		{#if query.trim().length < 2}
			<Empty.Root class="border-none py-10">
				<Empty.Header>
					<Empty.Title>Search your chats</Empty.Title>
					<Empty.Description
						>Type at least 2 characters to search through your chats and messages</Empty.Description
					>
				</Empty.Header>
			</Empty.Root>
		{:else if loading}
			<Empty.Root class="border-none py-10">
				<Empty.Header>
					<Empty.Description>Searching...</Empty.Description>
				</Empty.Header>
			</Empty.Root>
		{:else if threadResults.length === 0 && messageResults.length === 0}
			<Empty.Root class="border-none py-10">
				<Empty.Header>
					<Empty.Title>No results found</Empty.Title>
					<Empty.Description>Try a different search term</Empty.Description>
				</Empty.Header>
			</Empty.Root>
		{/if}

		{#if threadResults.length > 0}
			<Command.Group heading="Chats">
				{#each threadResults as thread}
					<Command.Item
						value="thread-{thread.id}"
						onSelect={() => selectThread(thread.id)}
					>
						<FileTextIcon class="size-4 text-muted-foreground" />
						<span>{thread.title}</span>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}

		{#if messageResults.length > 0}
			<Command.Group heading="Messages">
				{#each messageResults as message}
					<Command.Item
						value="msg-{message.id}"
						onSelect={() => selectMessage(message.thread_id)}
					>
						<MessageSquareIcon class="size-4 text-muted-foreground" />
						<div class="flex flex-col gap-0.5 min-w-0">
							<span class="text-xs text-muted-foreground">{message.message_type}</span
							>
							<span class="line-clamp-2 text-sm"
								>{@html sanitizeSnippet(message.snippet)}</span
							>
						</div>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>

<style>
	:global([data-slot='dialog-content']:has([data-slot='command'])) {
		top: 33% !important;
		translate: -50% 0 !important;
	}

	:global([data-command-item] mark) {
		padding: 0 2px;
		border-radius: 2px;
		background-color: var(--accent);
		color: var(--accent-foreground);
	}
</style>
