<script lang="ts">
	import { goto } from '$app/navigation';
	import {
		commands,
		events,
		type BrowserExtensionState,
		type ContextChip,
	} from '$lib/bindings/specta.bindings.js';
	import { unwrap } from '$lib/bindings/result.js';
	import { buildSuggestions } from '$lib/chat/suggestions.js';
	import { TIMELINE_SERVICE } from '$lib/services/timeline-service.svelte.js';
	import { MessageList, ChatPromptInput, middleTruncate } from '@eurora/chat';
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

	let { data } = $props();

	const chatService = inject(CHAT_SERVICE);
	const timelineService = inject(TIMELINE_SERVICE);
	let assets = $state<ContextChip[] | null>(null);

	const threadId = $derived(data.threadId);
	const latestTimelineItem = $derived(timelineService.latest);
	const focusedProcessName = $derived(latestTimelineItem?.processName ?? '');
	const focusedProcessId = $derived(latestTimelineItem?.processId ?? 0);

	let extensionState = $state<BrowserExtensionState | null>(null);
	let unlistenStatus: (() => void) | null = null;

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
		extensionState = null;
		if (!processName) return;

		// Capture the current process name so racing responses from a previous
		// focused process can't overwrite state for the current one.
		commands
			.systemGetBrowserExtensionState(processName)
			.then((state) => {
				if (focusedProcessName !== processName) return;
				extensionState = state;
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

	function handleRegenerate(messageId: string) {
		chatService.regenerateAi(messageId).catch((e) => toast.error(String(e)));
	}

	async function installExtension(installUrl: string) {
		const pid = focusedProcessId;

		try {
			if (pid > 0) {
				unwrap(await commands.systemOpenUrlInBrowser(pid, installUrl));
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
			await openExternal(installUrl);
		} catch (err) {
			toast.error(`Failed to open extension page: ${err}`);
		}
	}

	async function openExtensionSettings() {
		try {
			unwrap(await commands.systemOpenBrowserExtensionSettings(focusedProcessName));
		} catch (err) {
			toast.error(`Failed to open extension settings: ${err}`);
		}
	}

	onMount(() => {
		events.timelineAssetsEvent.listen((e) => {
			assets = e.payload;
		});

		// Seed the initial chip state so the suggestions row doesn't render
		// stale "no active page" suggestions before the first event arrives.
		// Without this, the reactive `$derived` below would briefly show the
		// no-context suggestion and then swap it out under a clicking user.
		commands
			.systemListActivities()
			.then((result) => {
				if (assets === null) assets = unwrap(result);
			})
			.catch((e) => toast.error(String(e)));

		events.browserExtensionStatusChanged
			.listen((event) => {
				const status = event.payload;
				if (status.process_name !== focusedProcessName) return;
				extensionState = status.state;
			})
			.then((unlisten) => {
				unlistenStatus = unlisten;
			})
			.catch((e) => {
				toast.error(`Failed to subscribe to browser extension status: ${e}`);
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
			{#if latestTimelineItem?.iconBase64}
				<Empty.Title>Currently on</Empty.Title>
				<Empty.Media variant="icon" class="bg-transparent">
					<img src={latestTimelineItem.iconBase64} alt="" class="size-full" />
				</Empty.Media>
			{:else}
				<Empty.Title>No messages yet</Empty.Title>
			{/if}
		</Empty.Header>
		{#if extensionState?.kind === 'not_installed'}
			{@const installUrl = extensionState.install_url}
			<Button variant="outline" size="sm" onclick={() => installExtension(installUrl)}>
				Install Eurora extension
				<ExternalLink class="size-4" />
			</Button>
		{:else if extensionState?.kind === 'disabled'}
			<Empty.Description>
				The Eurora extension is installed in Safari but currently turned off.
			</Empty.Description>
			<Button variant="outline" size="sm" onclick={openExtensionSettings}>
				Enable Eurora in Safari
				<ExternalLink class="size-4" />
			</Button>
		{:else if extensionState?.kind === 'not_discovered'}
			<Empty.Description>
				Open Eurora once on this Mac, then enable the extension in Safari → Settings →
				Extensions.
			</Empty.Description>
			<Button variant="outline" size="sm" onclick={openExtensionSettings}>
				Open Safari Extension Settings
				<ExternalLink class="size-4" />
			</Button>
		{/if}
	</Empty.Root>
{/snippet}

<div class="flex h-full flex-col overflow-hidden">
	<MessageList
		onCopy={handleCopy}
		onEdit={handleEdit}
		onRegenerate={handleRegenerate}
		{emptyState}
	/>
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
