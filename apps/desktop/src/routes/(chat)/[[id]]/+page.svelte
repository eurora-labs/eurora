<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { buildSuggestions } from '$lib/chat/suggestions.js';
	import { TIMELINE_SERVICE } from '$lib/services/timeline-service.svelte.js';
	import { MessageList, MessageGraph, ChatPromptInput, middleTruncate } from '@eurora/chat';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Attachment from '@eurora/ui/components/ai-elements/attachments/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import ExternalLink from '@lucide/svelte/icons/external-link';
	import { writeText } from '@tauri-apps/plugin-clipboard-manager';
	import { open as openExternal } from '@tauri-apps/plugin-shell';
	import { onMount, onDestroy } from 'svelte';
	import { toast } from 'svelte-sonner';
	import type { ContextChip } from '$lib/bindings/bindings.js';

	let { data } = $props();

	const taurpc = inject(TAURPC_SERVICE);
	const chatService = inject(CHAT_SERVICE);
	const timelineService = inject(TIMELINE_SERVICE);
	let assets = $state<ContextChip[] | null>(null);

	const threadId = $derived(data.threadId);
	const hasMessages = $derived((chatService.activeThread?.messages.length ?? 0) > 0);
	const latestTimelineItem = $derived(timelineService.latest);
	const focusedProcessName = $derived(latestTimelineItem?.process_name ?? '');
	const focusedProcessId = $derived(latestTimelineItem?.process_id ?? 0);

	let extensionUrl = $state<string | null>(null);
	let extensionConnected = $state(false);
	let unlistenStatus: (() => void) | null = null;

	const showInstallExtension = $derived(
		!!extensionUrl && !extensionConnected && focusedProcessName !== '',
	);

	$effect(() => {
		if (threadId) {
			chatService.activeThreadId = threadId;
			chatService.loadMessages(threadId);
		}
	});

	$effect(() => {
		const newThread = chatService.newThread;
		if (newThread) {
			chatService.newThread = undefined;
			goto(`/${newThread.id}`, { replaceState: true, keepFocus: true });
		}
	});

	$effect(() => {
		const processName = focusedProcessName;
		extensionUrl = null;
		extensionConnected = false;
		if (!processName) return;

		// Capture the current process name so racing responses from a previous
		// focused process can't overwrite state for the current one.
		Promise.all([
			taurpc.system.get_browser_extension_url(processName),
			taurpc.system.is_app_bridge_client_connected(processName),
		])
			.then(([url, connected]) => {
				if (focusedProcessName !== processName) return;
				extensionUrl = url;
				extensionConnected = connected;
			})
			.catch((e) => {
				if (focusedProcessName !== processName) return;
				toast.error(`Failed to resolve browser extension state: ${e}`);
			});
	});

	function handleCopy(content: string) {
		writeText(content).catch((e) => toast.error(`Failed to copy: ${e}`));
	}

	function handleSubmit(text: string) {
		chatService.sendMessage(text, assets ?? []).catch((e) => toast.error(String(e)));
	}

	function removeAsset(id: string) {
		if (!assets) return;
		assets = assets.filter((a) => a.id !== id);
	}

	function handleEdit(messageId: string, newText: string) {
		chatService.editMessage(messageId, newText).catch((e) => toast.error(String(e)));
	}

	function handleGraphNavigate(messageId: string) {
		if (!threadId) return;
		chatService.switchBranch(threadId, messageId, 0).catch((e) => toast.error(String(e)));
		chatService.viewMode = 'list';
	}

	async function installExtension() {
		const url = extensionUrl;
		const pid = focusedProcessId;
		if (!url) return;

		try {
			if (pid > 0) {
				await taurpc.system.open_url_in_browser(pid, url);
				return;
			}
		} catch (err) {
			// Falls through to the OS-default fallback below. The targeted
			// browser may have exited between the timeline event and the click,
			// or the spawn may have been refused; either way the user is better
			// served by *some* browser opening the page than by an error toast.
			console.warn('open_url_in_browser failed, falling back to default browser', err);
		}

		try {
			await openExternal(url);
		} catch (err) {
			toast.error(`Failed to open extension page: ${err}`);
		}
	}

	onMount(() => {
		taurpc.timeline.new_assets_event.on((chips) => {
			assets = chips;
		});

		// Seed the initial chip state so the suggestions row doesn't render
		// stale "no active page" suggestions before the first event arrives.
		// Without this, the reactive `$derived` below would briefly show the
		// no-context suggestion and then swap it out under a clicking user.
		taurpc.context_chip
			.get()
			.then((chips) => {
				if (assets === null) assets = chips;
			})
			.catch((e) => toast.error(String(e)));

		taurpc.system.app_bridge_client_status_changed
			.on((status) => {
				if (status.process_name !== focusedProcessName) return;
				extensionConnected = status.connected;
			})
			.then((unlisten) => {
				unlistenStatus = unlisten;
			})
			.catch((e) => {
				toast.error(`Failed to subscribe to app bridge client status: ${e}`);
			});
	});

	onDestroy(() => {
		unlistenStatus?.();
		unlistenStatus = null;
	});

	const suggestions = $derived(
		assets === null ? [] : buildSuggestions({ chips: assets, chatService, send: handleSubmit }),
	);
</script>

{#snippet emptyState()}
	<Empty.Root>
		<Empty.Header>
			{#if latestTimelineItem?.icon_base64}
				<Empty.Title>Currently on</Empty.Title>
				<Empty.Media variant="icon" class="bg-transparent">
					<img src={latestTimelineItem.icon_base64} alt="" class="size-full" />
				</Empty.Media>
			{:else}
				<Empty.Title>No messages yet</Empty.Title>
			{/if}
		</Empty.Header>
		{#if showInstallExtension}
			<Button variant="outline" size="sm" onclick={installExtension}>
				Install Eurora extension
				<ExternalLink class="size-4" />
			</Button>
		{/if}
	</Empty.Root>
{/snippet}

<div class="flex h-full flex-col overflow-hidden">
	{#if chatService.viewMode === 'graph' && hasMessages}
		<MessageGraph onMessageDblClick={handleGraphNavigate} class="min-h-0 flex-1" />
	{:else}
		<MessageList onCopy={handleCopy} onEdit={handleEdit} {emptyState} />
	{/if}
	<ChatPromptInput onSubmit={handleSubmit} {suggestions}>
		{#snippet header()}
			{#if assets && assets.length > 0}
				<Attachment.Root variant="inline">
					{#each assets as asset (asset.id)}
						<Attachment.Item
							data={{
								type: 'file',
								id: asset.id,
								filename: middleTruncate(asset.name),
							}}
							onRemove={() => removeAsset(asset.id)}
						>
							<Attachment.Preview />
							<Attachment.Info />
							<Attachment.Remove />
						</Attachment.Item>
					{/each}
				</Attachment.Root>
			{/if}
		{/snippet}
	</ChatPromptInput>
</div>
