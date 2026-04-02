<script lang="ts">
	import { CHAT_SERVICE } from '$lib/services/chat/chat-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import * as InfiniteList from '@eurora/ui/custom-components/infinite-list/index';
	import EllipsisIcon from '@lucide/svelte/icons/ellipsis';
	import Trash2Icon from '@lucide/svelte/icons/trash-2';
	import { toast } from 'svelte-sonner';

	interface Props {
		onThreadSelect?: (threadId: string) => void;
	}

	let { onThreadSelect }: Props = $props();

	const chatService = inject(CHAT_SERVICE);

	let deleteThreadId: string | null = $state(null);
	let deleteThreadTitle = $derived.by(() => {
		if (!deleteThreadId) return '';
		return (
			chatService.threads.find((t) => t.thread.id === deleteThreadId)?.thread.title ??
			'New Thread'
		);
	});

	async function deleteThread() {
		if (!deleteThreadId) return;
		const id = deleteThreadId;
		deleteThreadId = null;
		try {
			await chatService.deleteThread(id);
			if (chatService.activeThreadId === null) {
				onThreadSelect?.('');
			}
		} catch (error) {
			console.error('Failed to delete thread:', error);
			toast.error('Failed to delete chat');
		}
	}
</script>

<InfiniteList.Root
	items={chatService.threads}
	label="Chats"
	loading={chatService.loading}
	loadingMore={chatService.loadingMore}
	hasMore={chatService.hasMore}
	onLoadMore={() => chatService.loadMore()}
>
	{#snippet empty()}
		<Empty.Root>
			<Empty.Header>
				<Empty.Title>No chats yet</Empty.Title>
			</Empty.Header>
		</Empty.Root>
	{/snippet}
	{#snippet children(item)}
		<Sidebar.MenuItem>
			<Sidebar.MenuButton
				isActive={item.thread.id === chatService.activeThreadId}
				onclick={() => {
					if (!item.thread.id) {
						toast.error("Something went wrong: this thread doesn't exist.");
						return;
					}
					chatService.activeThreadId = item.thread.id;
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
							deleteThreadId = item.thread.id ?? null;
						}}
					>
						<Trash2Icon />
						<span>Delete</span>
					</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		</Sidebar.MenuItem>
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
			<Button variant="destructive" onclick={deleteThread}>Delete</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
