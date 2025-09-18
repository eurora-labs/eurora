<script module lang="ts">
	import { CommandItem } from '$lib/launcher/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components/Launcher/CommandItem',
		component: CommandItem,
		parameters: {
			layout: 'centered',
			docs: {
				description: {
					component:
						'Individual command items that can be selected within a command palette.',
				},
			},
		},
		argTypes: {
			disabled: {
				control: 'boolean',
				description: 'Whether the item is disabled',
			},
		},
		args: {
			disabled: false,
		},
	});
</script>

<script lang="ts">
	import * as Command from '$lib/launcher/index.js';
	import { StorybookContainer } from '@eurora/ui/storybook-container/index.js';
	import {
		Calendar,
		Calculator,
		Settings,
		Search,
		FileText,
		Mail,
		User,
		Home,
		Folder,
	} from '@lucide/svelte';
</script>

<Story name="Basic Items">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group>
					<Command.Item>
						<span>Simple Item</span>
					</Command.Item>
					<Command.Item>
						<span>Another Item</span>
					</Command.Item>
					<Command.Item>
						<span>Third Item</span>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="With Icons">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group>
					<Command.Item>
						<Calendar class="mr-2 h-4 w-4" />
						<span>Calendar</span>
					</Command.Item>
					<Command.Item>
						<Calculator class="mr-2 h-4 w-4" />
						<span>Calculator</span>
					</Command.Item>
					<Command.Item>
						<Settings class="mr-2 h-4 w-4" />
						<span>Settings</span>
					</Command.Item>
					<Command.Item>
						<Search class="mr-2 h-4 w-4" />
						<span>Search</span>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="With Shortcuts">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group>
					<Command.Item>
						<Calendar class="mr-2 h-4 w-4" />
						<span>Calendar</span>
						<Command.Shortcut>⌘C</Command.Shortcut>
					</Command.Item>
					<Command.Item>
						<Calculator class="mr-2 h-4 w-4" />
						<span>Calculator</span>
						<Command.Shortcut>⌘K</Command.Shortcut>
					</Command.Item>
					<Command.Item>
						<Settings class="mr-2 h-4 w-4" />
						<span>Settings</span>
						<Command.Shortcut>⌘,</Command.Shortcut>
					</Command.Item>
					<Command.Item>
						<Search class="mr-2 h-4 w-4" />
						<span>Global Search</span>
						<Command.Shortcut>⌘/</Command.Shortcut>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="Colored Icons">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group>
					<Command.Item>
						<Home class="mr-2 h-4 w-4 text-blue-500" />
						<span>Home</span>
					</Command.Item>
					<Command.Item>
						<FileText class="mr-2 h-4 w-4 text-green-500" />
						<span>Documents</span>
					</Command.Item>
					<Command.Item>
						<Mail class="mr-2 h-4 w-4 text-red-500" />
						<span>Mail</span>
					</Command.Item>
					<Command.Item>
						<User class="mr-2 h-4 w-4 text-purple-500" />
						<span>Profile</span>
					</Command.Item>
					<Command.Item>
						<Folder class="mr-2 h-4 w-4 text-yellow-500" />
						<span>Folder</span>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="Disabled Items">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group>
					<Command.Item>
						<Calendar class="mr-2 h-4 w-4" />
						<span>Available Item</span>
					</Command.Item>
					<Command.Item disabled>
						<Calculator class="mr-2 h-4 w-4" />
						<span>Disabled Item</span>
					</Command.Item>
					<Command.Item>
						<Settings class="mr-2 h-4 w-4" />
						<span>Another Available Item</span>
					</Command.Item>
					<Command.Item disabled>
						<Search class="mr-2 h-4 w-4" />
						<span>Another Disabled Item</span>
						<Command.Shortcut>⌘/</Command.Shortcut>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

{#snippet template(args: { disabled: boolean }, _context: StoryContext<typeof Story>)}
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group>
					<Command.Item disabled={args.disabled}>
						<Calendar class="mr-2 h-4 w-4" />
						<span>{args.disabled ? 'Disabled Item' : 'Interactive Item'}</span>
						<Command.Shortcut>⌘I</Command.Shortcut>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
{/snippet}
