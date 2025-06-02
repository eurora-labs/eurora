<script module lang="ts">
	import { CommandItem } from '$lib/custom-components/launcher/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components/Launcher/CommandItem',
		component: CommandItem,
		parameters: {
			layout: 'centered',
			docs: {
				description: {
					component:
						'Individual command items that can be selected within a command palette.'
				}
			}
		},
		argTypes: {
			disabled: {
				control: 'boolean',
				description: 'Whether the item is disabled'
			}
		},
		args: {
			disabled: false
		}
	});
</script>

<script lang="ts">
	import * as Command from '$lib/custom-components/launcher/index.js';
	import {
		Calendar,
		Calculator,
		Settings,
		Search,
		FileText,
		Mail,
		User,
		Home,
		Folder
	} from '@lucide/svelte';
</script>

<!-- Basic Items -->
<Story name="Basic Items">
	<div class="w-[350px]">
		<Command.Root class="rounded-lg border shadow-md">
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
	</div>
</Story>

<!-- Items with Icons -->
<Story name="With Icons">
	<div class="w-[350px]">
		<Command.Root class="rounded-lg border shadow-md">
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
	</div>
</Story>

<!-- Items with Shortcuts -->
<Story name="With Shortcuts">
	<div class="w-[350px]">
		<Command.Root class="rounded-lg border shadow-md">
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
	</div>
</Story>

<!-- Colored Icons -->
<Story name="Colored Icons">
	<div class="w-[350px]">
		<Command.Root class="rounded-lg border shadow-md">
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
	</div>
</Story>

<!-- Disabled Items -->
<Story name="Disabled Items">
	<div class="w-[350px]">
		<Command.Root class="rounded-lg border shadow-md">
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
	</div>
</Story>

<!-- Interactive Example -->
{#snippet template({ ...args }: Args<typeof Story>, _context: StoryContext<typeof Story>)}
	<div class="w-[350px]">
		<Command.Root class="rounded-lg border shadow-md">
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
	</div>
{/snippet}
