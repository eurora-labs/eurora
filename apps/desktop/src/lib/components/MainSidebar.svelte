<script lang="ts">
	import { goto } from '$app/navigation';
	import { type TimelineAppEvent } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { THREAD_SERVICE } from '$lib/services/thread-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import * as Empty from '@eurora/ui/components/empty/index';
	import { useSidebar } from '@eurora/ui/components/sidebar/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import * as InfiniteList from '@eurora/ui/custom-components/infinite-list/index';
	import * as Timeline from '@eurora/ui/custom-components/timeline/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import LogoutIcon from '@lucide/svelte/icons/log-out';
	import PowerIcon from '@lucide/svelte/icons/power';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import SquarePenIcon from '@lucide/svelte/icons/square-pen';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);
	const threadService = inject(THREAD_SERVICE);
	const sidebarState = useSidebar();
	let timelineItems: TimelineAppEvent[] = $state([]);

	let quitDialogOpen = $state(false);
	let username = $state('');
	let visibleTimelineItems = $derived.by(() => {
		const limit = sidebarState.open ? 3 : 1;
		return timelineItems.slice(-limit);
	});

	function getFirstLetterAndCapitalize(name: string) {
		if (!name) return '';
		return name.charAt(0).toUpperCase();
	}

	onMount(() => {
		const unlistenPromises: Promise<() => void>[] = [];

		unlistenPromises.push(
			taurpc.timeline.new_app_event.on((e) => {
				if (timelineItems.length >= 5) {
					timelineItems.shift();
				}
				timelineItems.push(e);
			}),
		);

		taurpc.auth
			.is_authenticated()
			.then((isAuthenticated) => {
				if (!isAuthenticated) {
					threadService.loading = false;
					return;
				}
				taurpc.auth.get_username().then((name) => {
					username = name;
				});
				threadService.init();
			})
			.catch((error) => {
				goto('/onboarding');
				console.error('Failed to check authentication:', error);
			});

		return () => {
			threadService.destroy();
			for (const p of unlistenPromises) {
				p.then((unlisten) => unlisten());
			}
		};
	});

	async function createChat() {
		threadService.createThread();
		goto('/');
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
				<EuroraLogo class="size-7" onclick={() => sidebarState.setOpen(true)} />
			</div>

			{#if sidebarState.open}
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
		{#if sidebarState.open}
			<InfiniteList.Root
				items={threadService.threads}
				label="Chats"
				loading={threadService.loading}
				loadingMore={threadService.loadingMore}
				hasMore={threadService.hasMore}
				onLoadMore={() => threadService.loadMore()}
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
							isActive={item.id === threadService.activeThreadId}
							onclick={() => {
								if (!item.id) {
									toast.error("Something went wrong: this thread doesn't exist.");
									return;
								}
								goto(`/${item.id}`);
							}}
						>
							{#snippet child({ props })}
								<a {...props}>
									<span>{item.title ?? 'New Thread'}</span>
								</a>
							{/snippet}
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				{/snippet}
			</InfiniteList.Root>
		{/if}
	</Sidebar.Content>
	{#if visibleTimelineItems.length > 0}
		<div class="px-2 py-2">
			<Timeline.Root class="w-full" defaultOpen={false}>
				{#each visibleTimelineItems as item, i}
					<Timeline.Item
						color={item.color}
						iconBg={item.icon_bg}
						highlighted={i === visibleTimelineItems.length - 1}
						iconSrc={item.icon_base64}
						name={item.name}
					/>
				{/each}
			</Timeline.Root>
		</div>
	{/if}
	<Sidebar.Footer>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				{#snippet child({ props })}
					<Button
						{...props}
						variant="ghost"
						class="flex items-center gap-2 min-w-0 h-auto px-1 py-1 w-full justify-start"
					>
						<div
							class="flex size-7 shrink-0 items-center justify-center rounded-full bg-sidebar-accent text-sidebar-accent-foreground text-xs font-medium"
						>
							{getFirstLetterAndCapitalize(username)}
						</div>
						{#if sidebarState.open}
							<span class="truncate text-sm flex-1 text-left">{username}</span>
							<ChevronUpIcon class="size-4 shrink-0" />
						{/if}
					</Button>
				{/snippet}
			</DropdownMenu.Trigger>
			<DropdownMenu.Content side="top" align="start" class="w-56">
				<DropdownMenu.Item onclick={() => goto('/settings')}>
					<SettingsIcon />
					<span>Settings</span>
				</DropdownMenu.Item>
				<DropdownMenu.Sub>
					<DropdownMenu.SubTrigger>
						<PowerIcon />
						<span>Power</span>
					</DropdownMenu.SubTrigger>
					<DropdownMenu.SubContent class="w-40">
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
					</DropdownMenu.SubContent>
				</DropdownMenu.Sub>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
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
