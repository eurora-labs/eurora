<script lang="ts">
	import { ThreadMessages } from '$lib/services/chat/chat-service.svelte.js';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import * as InfiniteList from '@eurora/ui/custom-components/infinite-list/index';
	import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import { toast } from 'svelte-sonner';
	import type { Snippet } from 'svelte';

	interface Props {
		threads: ThreadMessages[];
		loading: boolean;
		loadingMore: boolean;
		hasMore: boolean;
		onLoadMore: () => void;
		activeThreadId: string | undefined;
		onThreadSelect?: (threadId: string) => void;
		onThreadDelete: (threadId: string) => Promise<void>;
		/**
		 * Section label displayed at the top of the main (paginated) list.
		 * Defaults to "Chats".
		 */
		label?: string;
		/**
		 * Override the default "No chats yet" empty state of the main list.
		 */
		emptyState?: Snippet;
		/**
		 * Optional pinned group rendered above the main list. Used by the
		 * desktop sidebar for the timeline-rail per-app filter: even when
		 * the bucket is empty, the labelled group still renders with a
		 * "No chats" placeholder so the user can see the filter is active.
		 */
		pinnedLabel?: string;
		/**
		 * Threads to render inside the pinned group. When `undefined` or
		 * empty, the group still renders with `pinnedEmptyText` so the
		 * label / filter context stays visible.
		 */
		pinnedThreads?: ThreadMessages[];
		/**
		 * Placeholder rendered when `pinnedThreads` is empty. Defaults to
		 * "No chats".
		 */
		pinnedEmptyText?: string;
		/**
		 * CSS colour for the left-edge accent strip on pinned items.
		 * Defaults to `--sidebar-primary`.
		 */
		pinnedAccentColor?: string;
	}

	let {
		threads,
		loading,
		loadingMore,
		hasMore,
		onLoadMore,
		activeThreadId,
		onThreadSelect,
		onThreadDelete,
		label = 'Chats',
		emptyState,
		pinnedLabel,
		pinnedThreads,
		pinnedEmptyText = 'No chats',
		pinnedAccentColor,
	}: Props = $props();

	let deleteThreadId: string | null = $state(null);
	let deleteThreadTitle = $derived.by(() => {
		if (!deleteThreadId) return '';
		// The thread being deleted may live in either group, so search both.
		const haystack = [...(pinnedThreads ?? []), ...threads];
		return haystack.find((t) => t.thread.id === deleteThreadId)?.thread.title ?? 'New Thread';
	});

	async function confirmDelete(): Promise<void> {
		if (!deleteThreadId) return;
		const id = deleteThreadId;
		deleteThreadId = null;
		try {
			await onThreadDelete(id);
		} catch (error) {
			console.error('Failed to delete thread:', error);
			toast.error('Failed to delete chat');
		}
	}
</script>

{#snippet defaultEmpty()}
	<Empty.Root>
		<Empty.Header>
			<Empty.Title>No chats yet</Empty.Title>
		</Empty.Header>
	</Empty.Root>
{/snippet}

{#snippet threadItem(item: ThreadMessages, pinned: boolean)}
	<Sidebar.MenuItem
		class={pinned ? 'sidebar-threads-list-pinned' : undefined}
		style={pinned && pinnedAccentColor !== undefined
			? `--threads-list-pinned-color: ${pinnedAccentColor};`
			: undefined}
	>
		<Sidebar.MenuButton
			isActive={item.thread.id === activeThreadId}
			onclick={() => {
				onThreadSelect?.(item.thread.id);
			}}
		>
			{#snippet child({ props })}
				<a {...props}>
					<span>{item.thread.title ?? 'New Thread'}</span>
				</a>
			{/snippet}
		</Sidebar.MenuButton>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Sidebar.MenuAction {...props} showOnHover>
						<EllipsisIcon />
						<span class="sr-only">More</span>
					</Sidebar.MenuAction>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content side="right" align="start">
				<DropdownMenu.Item
					onclick={() => {
						deleteThreadId = item.thread.id;
					}}
				>
					<Trash2Icon />
					<span>Delete</span>
				</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
	</Sidebar.MenuItem>
{/snippet}

{#if pinnedLabel}
	<Sidebar.Group>
		<Sidebar.GroupLabel>{pinnedLabel}</Sidebar.GroupLabel>
		<Sidebar.GroupContent>
			{#if pinnedThreads && pinnedThreads.length > 0}
				<Sidebar.Menu>
					{#each pinnedThreads as item (item.thread.id)}
						{@render threadItem(item, true)}
					{/each}
				</Sidebar.Menu>
			{:else}
				<div class="text-muted-foreground px-2 py-1.5 text-xs">{pinnedEmptyText}</div>
			{/if}
		</Sidebar.GroupContent>
	</Sidebar.Group>
{/if}

<InfiniteList.Root
	items={threads}
	{label}
	{loading}
	{loadingMore}
	{hasMore}
	{onLoadMore}
	empty={emptyState ?? defaultEmpty}
>
	{#snippet children(item)}
		{@render threadItem(item, false)}
	{/snippet}
</InfiniteList.Root>

<Dialog.Root
	open={deleteThreadId !== null}
	onOpenChange={(open) => {
		if (!open) deleteThreadId = null;
	}}
>
	<Dialog.Content class="sm:max-w-100">
		<Dialog.Header>
			<Dialog.Title>Delete Chat</Dialog.Title>
			<Dialog.Description>
				Chat <span class="font-bold text-foreground">"{deleteThreadTitle}"</span> will be permanently
				deleted along with all its messages. This action cannot be undone.
			</Dialog.Description>
		</Dialog.Header>
		<Dialog.Footer class="gap-2">
			<Dialog.Close class={buttonVariants({ variant: 'outline' })}>Cancel</Dialog.Close>
			<Button variant="destructive" onclick={confirmDelete}>Delete</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<style>
	:global(.sidebar-threads-list-pinned) {
		position: relative;
	}
	:global(.sidebar-threads-list-pinned::before) {
		position: absolute;
		width: 2px;
		inset-block: 0.25rem;
		inset-inline-start: 0;
		border-radius: 9999px;
		background-color: var(--threads-list-pinned-color, var(--sidebar-primary));
		content: '';
		pointer-events: none;
	}
</style>
