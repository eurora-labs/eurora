<script lang="ts">
	import { goto } from '$app/navigation';
	import { type ThreadView, type TimelineAppEvent } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import { useSidebar } from '@eurora/ui/components/sidebar/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import { Spinner } from '@eurora/ui/components/spinner/index';
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
	let threads: ThreadView[] = $state([]);
	let timelineItems: TimelineAppEvent[] = $state([]);

	let sidebarState: ReturnType<typeof useSidebar> | undefined = $state(undefined);
	let chatsLoading = $state(true);
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
					chatsLoading = false;
					return;
				}
				taurpc.auth.get_username().then((name) => {
					username = name;
				});
				taurpc.thread.list(10, 0).then((res) => {
					threads = res;
					chatsLoading = false;
				});

				unlistenPromises.push(
					taurpc.thread.new_thread_added.on((thread) => {
						if (!threads.some((c) => c.id === thread.id)) {
							threads = [thread, ...threads];
						}
					}),
				);

				unlistenPromises.push(
					taurpc.thread.thread_title_changed.on((thread) => {
						for (const c of threads) {
							if (c.id === thread.id) {
								c.title = thread.title;
							}
						}
					}),
				);
			})
			.catch((error) => {
				chatsLoading = false;
				goto('/onboarding');

				console.error('Failed to check authentication:', error);
			});

		return () => {
			for (const p of unlistenPromises) {
				p.then((unlisten) => unlisten());
			}
		};
	});

	async function createChat() {
		await taurpc.thread.create_empty_thread().catch((error) => {
			console.error('Failed to create thread:', error);
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

	async function switchThread(id: string) {
		await taurpc.thread.switch_thread(id);
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
		{#if sidebarState?.open}
			<Sidebar.Group>
				<Sidebar.GroupLabel>Chats</Sidebar.GroupLabel>

				<Sidebar.GroupContent>
					{#if chatsLoading}
						<div class="flex items-center justify-center py-4">
							<Spinner />
						</div>
					{:else if threads.length === 0}
						<p class="px-3 py-4 text-sm text-muted-foreground text-center">
							No Chats Yet
						</p>
					{:else}
						<Sidebar.Menu>
							{#each threads as item (item.id)}
								<Sidebar.MenuItem>
									<Sidebar.MenuButton
										onclick={() => {
											switchThread(item.id ?? '');
										}}
									>
										{#snippet child({ props })}
											<a {...props}>
												<span>{item.title ?? 'New Thread'}</span>
											</a>
										{/snippet}
									</Sidebar.MenuButton>
								</Sidebar.MenuItem>
							{/each}
						</Sidebar.Menu>
					{/if}
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
						{#if sidebarState?.open}
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
