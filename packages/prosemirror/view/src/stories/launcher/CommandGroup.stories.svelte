<script module lang="ts">
	import { CommandGroup } from '$lib/launcher/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components/Launcher/CommandGroup',
		component: CommandGroup,
		parameters: {
			layout: 'centered',
			docs: {
				description: {
					component: 'Groups related command items together with an optional heading.',
				},
			},
		},
		argTypes: {
			heading: {
				control: 'text',
				description: 'The heading text for the group',
			},
		},
		args: {
			heading: 'Applications',
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
		Music,
		Video,
		Image,
	} from '@lucide/svelte';
</script>

<Story name="Basic Group">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group heading="Applications">
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
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="Without Heading">
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
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="Multiple Groups">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group heading="Applications">
					<Command.Item>
						<Calendar class="mr-2 h-4 w-4" />
						<span>Calendar</span>
					</Command.Item>
					<Command.Item>
						<Calculator class="mr-2 h-4 w-4" />
						<span>Calculator</span>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Files">
					<Command.Item>
						<FileText class="mr-2 h-4 w-4" />
						<span>Documents</span>
					</Command.Item>
					<Command.Item>
						<Folder class="mr-2 h-4 w-4" />
						<span>Downloads</span>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Media">
					<Command.Item>
						<Music class="mr-2 h-4 w-4" />
						<span>Music</span>
					</Command.Item>
					<Command.Item>
						<Video class="mr-2 h-4 w-4" />
						<span>Videos</span>
					</Command.Item>
					<Command.Item>
						<Image class="mr-2 h-4 w-4" />
						<span>Photos</span>
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
				<Command.Group heading="Quick Actions">
					<Command.Item>
						<Home class="mr-2 h-4 w-4" />
						<span>Go Home</span>
						<Command.Shortcut>⌘H</Command.Shortcut>
					</Command.Item>
					<Command.Item>
						<Search class="mr-2 h-4 w-4" />
						<span>Global Search</span>
						<Command.Shortcut>⌘/</Command.Shortcut>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Settings">
					<Command.Item>
						<Settings class="mr-2 h-4 w-4" />
						<span>Preferences</span>
						<Command.Shortcut>⌘,</Command.Shortcut>
					</Command.Item>
					<Command.Item>
						<User class="mr-2 h-4 w-4" />
						<span>Profile</span>
						<Command.Shortcut>⌘P</Command.Shortcut>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="Different Headings">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group heading="🚀 Quick Actions">
					<Command.Item>
						<Home class="mr-2 h-4 w-4" />
						<span>Go Home</span>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="📁 Files & Folders">
					<Command.Item>
						<FileText class="mr-2 h-4 w-4" />
						<span>Documents</span>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="⚙️ System">
					<Command.Item>
						<Settings class="mr-2 h-4 w-4" />
						<span>Settings</span>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

{#snippet template(args: { heading: string }, _context: StoryContext<typeof Story>)}
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.List>
				<Command.Group heading={args.heading}>
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
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
{/snippet}
