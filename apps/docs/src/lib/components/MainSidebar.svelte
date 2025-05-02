<script lang="ts">
	import {
		Calendar,
		House,
		Inbox,
		Search,
		Settings,
		Smile,
		CreditCard,
		Calculator,
		User
	} from '@lucide/svelte';
	import {
		Sidebar,
		EurIcons,
		Dialog,
		Label,
		Input,
		buttonVariants,
		Button,
		Command
	} from '@eurora/ui';
	import { EuroraLogo } from '../../../../../packages/ui/src/custom-icons';

	// Menu items.
	const items = [
		{
			title: 'Chat 1',
			url: '#',
			icon: House
		},
		{
			title: 'Chat 2',
			url: '#',
			icon: Inbox
		},
		{
			title: 'Chat 3',
			url: '#',
			icon: Calendar
		},
		{
			title: 'Chat 4',
			url: '#',
			icon: Search
		},
		{
			title: 'Chat 5',
			url: '#',
			icon: Settings
		}
	];
</script>

<Sidebar.Root>
	<Sidebar.Header><EurIcons.EuroraLogo />Eurora Labs</Sidebar.Header>
	<Dialog.Root>
		<Dialog.Trigger class={buttonVariants({ variant: 'outline' })}><Search /> Search</Dialog.Trigger
		>
		<Dialog.Content>
			<!-- <Dialog.Header>
				<Dialog.Title>Edit profile</Dialog.Title>
				<Dialog.Description>
					Make changes to your profile here. Click save when you're done.
				</Dialog.Description>
			</Dialog.Header> -->
			<!-- <div class="grid gap-4 py-4">
				<div class="grid grid-cols-4 items-center gap-4">
					<Label for="name" class="text-right">Name</Label>
					<Input id="name" value="Pedro Duarte" class="col-span-3" />
				</div>
				<div class="grid grid-cols-4 items-center gap-4">
					<Label for="username" class="text-right">Username</Label>
					<Input id="username" value="@peduarte" class="col-span-3" />
				</div>
			</div> -->
			<Command.Root>
				<Command.Input placeholder="Search" />
				<Command.List>
					<Command.Empty>No results found.</Command.Empty>
					<Command.Group heading="Suggestions">
						<Command.Item>
							<Calendar />
							<span>Calendar</span>
						</Command.Item>
						<Command.Item>
							<Smile />
							<span>Search Emoji</span>
						</Command.Item>
						<Command.Item disabled>
							<Calculator />
							<span>Calculator</span>
						</Command.Item>
					</Command.Group>
					<Command.Separator />
					<Command.Group heading="Settings">
						<Command.Item>
							<User />
							<span>Profile</span>
							<Command.Shortcut>⌘P</Command.Shortcut>
						</Command.Item>
						<Command.Item>
							<CreditCard />
							<span>Billing</span>
							<Command.Shortcut>⌘B</Command.Shortcut>
						</Command.Item>
						<Command.Item>
							<Settings />
							<span>Settings</span>
							<Command.Shortcut>⌘S</Command.Shortcut>
						</Command.Item>
					</Command.Group>
				</Command.List>
			</Command.Root>
			<!-- <Dialog.Footer>
				<Button type="submit">Save changes</Button>
			</Dialog.Footer> -->
		</Dialog.Content>
	</Dialog.Root>
	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>Welcome</Sidebar.MenuButton>
					</Sidebar.MenuItem>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>Getting Started</Sidebar.MenuButton>
					</Sidebar.MenuItem>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>Why Eurora</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
		<Sidebar.Group>
			<Sidebar.GroupLabel>Components</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					<Sidebar.MenuItem>
						<Sidebar.MenuButton>
							{#snippet child({ props })}
								<a href="/components/launcher" {...props}> <span>Launcher</span> </a>
							{/snippet}
						</Sidebar.MenuButton>
					</Sidebar.MenuItem>
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
		<Sidebar.Group>
			<Sidebar.GroupLabel>Application</Sidebar.GroupLabel>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each items as item (item.title)}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton>
								{#snippet child({ props })}
									<a href={item.url} {...props}>
										<item.icon />
										<span>{item.title}</span>
									</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
	</Sidebar.Content>
	<Sidebar.Footer>
		<div class="flex w-full justify-end">
			<Sidebar.Trigger />
		</div>
	</Sidebar.Footer>
</Sidebar.Root>
