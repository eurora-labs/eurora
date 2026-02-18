<script lang="ts">
	import { goto } from '$app/navigation';
	import { type ConversationView, type TimelineAppEvent } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import { useSidebar } from '@eurora/ui/components/sidebar/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import * as Timeline from '@eurora/ui/custom-components/timeline/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import LogoutIcon from '@lucide/svelte/icons/log-out';
	import PowerIcon from '@lucide/svelte/icons/power';
	import SquarePenIcon from '@lucide/svelte/icons/square-pen';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);
	let conversations: ConversationView[] = $state([]);
	let timelineItems: TimelineAppEvent[] = $state([]);

	let sidebarState: ReturnType<typeof useSidebar> | undefined = $state(undefined);
	let quitDialogOpen = $state(false);
	let username = $state('');

	let visibleTimelineItems = $derived.by(() => {
		const limit = sidebarState?.open ? 3 : 1;
		return timelineItems.slice(-limit);
	});

	function getFirstLetterAndCapitalize(name: string) {
		if (!name) return '';
		return name.charAt(0).toUpperCase();
	}

	onMount(() => {
		sidebarState = useSidebar();

		taurpc.timeline.new_app_event.on((e) => {
			if (timelineItems.length >= 5) {
				timelineItems.shift();
			}
			timelineItems.push(e);
		});

		taurpc.auth
			.is_authenticated()
			.then((isAuthenticated) => {
				if (!isAuthenticated) return;
				taurpc.auth.get_username().then((name) => {
					username = name;
				});
				taurpc.conversation.list(10, 0).then((res) => {
					conversations = res;
				});

				taurpc.conversation.new_conversation_added.on((conversation) => {
					if (!conversations.some((c) => c.id === conversation.id)) {
						conversations = [conversation, ...conversations];
					}
				});

				taurpc.conversation.conversation_title_changed.on((conversation) => {
					for (const c of conversations) {
						if (c.id === conversation.id) {
							c.title = conversation.title;
						}
					}
				});
			})
			.catch((error) => {
				goto('/onboarding');

				console.error('Failed to check authentication:', error);
			});
	});

	async function createChat() {
		await taurpc.conversation.create_empty_conversation().catch((error) => {
			console.error('Failed to create conversation:', error);
			toast.error(`The app encountered the following error: ${error}`, {
				description: 'Please try again later.',
				duration: 5000,
				cancel: {
					label: 'Ok',
					onClick: () => {},
				},
			});
		});
	}

	async function switchConversation(id: string) {
		await taurpc.conversation.switch_conversation(id);
	}

	async function quit() {
		quitDialogOpen = false;
		taurpc.system.quit().catch((error) => {
			console.error('Failed to quit application:', error);
			toast.error(`The app encountered the following error: ${error}`, {
				description: 'Please quit manually from the tray menu.',
				duration: 5000,
				cancel: {
					label: 'Ok',
					onClick: () => {},
				},
			});
			console.error('Failed to quit application:', error);
		});
	}
</script>

<Sidebar.Root collapsible="icon" class="border-none">
	<Sidebar.Header>
		<div class="flex items-center justify-between">
			<div class="flex items-center gap-2">
				<EuroraLogo class="size-7" onclick={() => sidebarState?.setOpen(true)} />
			</div>

			{#if sidebarState?.open}
				<Sidebar.Trigger />
			{/if}
		</div>
	</Sidebar.Header>
	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton onclick={() => createChat()}>
							<SquarePenIcon />
							<span>New chat</span>
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
		{#if conversations.length > 0 && sidebarState?.open}
			<Sidebar.Group>
				<Sidebar.GroupLabel>Chats</Sidebar.GroupLabel>

				<Sidebar.GroupContent>
					<Sidebar.Menu>
						{#each conversations as item (item.id)}
							<Sidebar.MenuItem>
								<Sidebar.MenuButton
									onclick={() => {
										switchConversation(item.id ?? '');
									}}
								>
									{#snippet child({ props })}
										<a {...props}>
											<span>{item.title ?? 'New Conversation'}</span>
										</a>
									{/snippet}
								</Sidebar.MenuButton>
							</Sidebar.MenuItem>
						{/each}
					</Sidebar.Menu>
				</Sidebar.GroupContent>
			</Sidebar.Group>
		{/if}
	</Sidebar.Content>
	{#if visibleTimelineItems.length > 0}
		<div class="px-2 py-2">
			<Timeline.Root class="w-full" defaultOpen={false}>
				{#each visibleTimelineItems as item, i}
					<Timeline.Item
						color={item.color}
						highlighted={i === visibleTimelineItems.length - 1}
					>
						{#if item.icon_base64}
							<img
								src={item.icon_base64}
								alt={item.name}
								class="w-8 h-8 bg-white rounded-full drop-shadow p-1"
							/>
						{:else}
							<div
								class="w-8 h-8 bg-white rounded-full drop-shadow p-1 flex items-center justify-center"
							>
								{getFirstLetterAndCapitalize(item.name)}
							</div>
						{/if}
					</Timeline.Item>
				{/each}
			</Timeline.Root>
		</div>
	{/if}
	<Sidebar.Footer>
		<div class="flex items-center justify-between gap-2">
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button
							{...props}
							variant="ghost"
							class="flex items-center gap-2 min-w-0 h-auto px-1 py-1 flex-1 justify-start"
						>
							<div
								class="flex size-7 shrink-0 items-center justify-center rounded-full bg-sidebar-accent text-sidebar-accent-foreground text-xs font-medium"
							>
								{getFirstLetterAndCapitalize(username)}
							</div>
							{#if sidebarState?.open}
								<span class="truncate text-sm">{username}</span>
							{/if}
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content side="top" align="start">
					<DropdownMenu.Item onclick={() => goto('/settings')}>
						<span>Settings</span>
					</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon" class="size-7 shrink-0">
							<PowerIcon class="size-4" />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Content side="top" align="end">
					<DropdownMenu.Item
						onclick={() => {
							taurpc.auth.logout().then(() => {
								goto('/onboarding');
							});
						}}
					>
						<LogoutIcon />
						<span>Log Out</span>
					</DropdownMenu.Item>
					<DropdownMenu.Item onclick={() => (quitDialogOpen = true)}>
						<PowerIcon />
						<span>Quit</span>
					</DropdownMenu.Item>
				</DropdownMenu.Content>
			</DropdownMenu.Root>
		</div>
	</Sidebar.Footer>
</Sidebar.Root>

<Dialog.Root bind:open={quitDialogOpen}>
	<Dialog.Content class="sm:max-w-100">
		<div class="flex gap-4">
			<div class="shrink-0">
				<EuroraLogo class="size-12" />
			</div>
			<div class="flex flex-col text-left">
				<Dialog.Header class="text-left">
					<Dialog.Title class="text-left">Quit Application</Dialog.Title>
					<Dialog.Description class="text-left">
						Are you sure you want to quit? Any unsaved changes will be lost.
					</Dialog.Description>
				</Dialog.Header>
			</div>
		</div>
		<Dialog.Footer class="gap-2 sm:gap-0">
			<Dialog.Close class={buttonVariants({ variant: 'outline' })}>Cancel</Dialog.Close>
			<Button variant="destructive" onclick={quit}>Quit</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
