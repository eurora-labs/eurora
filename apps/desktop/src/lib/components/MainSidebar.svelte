<script lang="ts">
	import { goto } from '$app/navigation';
	import { type ConversationView } from '$lib/bindings/bindings.js';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button, buttonVariants } from '@eurora/ui/components/button/index';
	import * as Dialog from '@eurora/ui/components/dialog/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import { useSidebar } from '@eurora/ui/components/sidebar/index';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import EuroraLogo from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import ChevronUpIcon from '@lucide/svelte/icons/chevron-up';
	import CircleUserRoundIcon from '@lucide/svelte/icons/circle-user-round';
	import LogoutIcon from '@lucide/svelte/icons/log-out';
	import PowerIcon from '@lucide/svelte/icons/power';
	import SettingsIcon from '@lucide/svelte/icons/settings';
	import SquarePenIcon from '@lucide/svelte/icons/square-pen';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpc = inject(TAURPC_SERVICE);
	let conversations: ConversationView[] = $state([]);

	let sidebarState: ReturnType<typeof useSidebar> | undefined = $state(undefined);
	let quitDialogOpen = $state(false);

	onMount(() => {
		sidebarState = useSidebar();

		taurpc.auth
			.is_authenticated()
			.then((isAuthenticated) => {
				if (!isAuthenticated) return;
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
	<Sidebar.Footer>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<DropdownMenu.Root>
					<DropdownMenu.Trigger>
						{#snippet child({ props })}
							<Sidebar.MenuButton
								{...props}
								class="data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground"
							>
								<CircleUserRoundIcon />
								<span>Profile</span>
								<ChevronUpIcon class="ml-auto" />
							</Sidebar.MenuButton>
						{/snippet}
					</DropdownMenu.Trigger>
					<DropdownMenu.Content side="top" class="w-(--bits-dropdown-menu-anchor-width)">
						<DropdownMenu.Item
							onclick={() => {
								taurpc.auth.logout().then(() => {
									goto('/onboarding');
								});
							}}
						>
							{#snippet child({ props })}
								<a {...props}>
									<LogoutIcon />
									<span>Log Out</span>
								</a>
							{/snippet}
						</DropdownMenu.Item>
						<DropdownMenu.Item onclick={() => (quitDialogOpen = true)}>
							{#snippet child({ props })}
								<a {...props}>
									<PowerIcon />
									<span>Quit</span>
								</a>
							{/snippet}
						</DropdownMenu.Item>
						<DropdownMenu.Item>
							{#snippet child({ props })}
								<a {...props} href="/settings">
									<SettingsIcon />
									<span>Settings</span>
								</a>
							{/snippet}
						</DropdownMenu.Item>
					</DropdownMenu.Content>
				</DropdownMenu.Root>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
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
