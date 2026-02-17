<script module lang="ts">
	import CommandRoot from '$lib/launcher/command.svelte';
	import * as Command from '$lib/launcher/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components/Launcher/Command',
		component: CommandRoot,
		parameters: {
			layout: 'centered',
			docs: {
				description: {
					component:
						'A command palette component for launching actions and navigating through the application.',
				},
			},
		},
		argTypes: {
			value: {
				control: 'text',
				description: 'The current search value',
			},
		},
		args: {
			value: '',
		},
	});
</script>

<script lang="ts">
	import { StorybookContainer } from '@eurora/ui/storybook-container/index.js';
	import {
		Search,
		Calculator,
		Calendar,
		Settings,
		User,
		FileText,
		Mail,
		Phone,
		Home,
		Folder,
		Image,
		Music,
		Video,
	} from '@lucide/svelte';
</script>

<Story name="Default">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.Input placeholder="Search" />
			<Command.List>
				<Command.Empty>No results found.</Command.Empty>
				<Command.Group heading="Suggestions">
					<Command.Item>
						<Calendar class="mr-2 h-4 w-4" />
						<span>Calendar</span>
					</Command.Item>
					<Command.Item>
						<Search class="mr-2 h-4 w-4" />
						<span>Search Emoji</span>
					</Command.Item>
					<Command.Item>
						<Calculator class="mr-2 h-4 w-4" />
						<span>Calculator</span>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Settings">
					<Command.Item>
						<Settings class="mr-2 h-4 w-4" />
						<span>Settings</span>
						<Command.Shortcut>⌘S</Command.Shortcut>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="With Groups">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.Input placeholder="Search" />
			<Command.List>
				<Command.Empty>No results found.</Command.Empty>
				<Command.Group heading="Applications">
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
						<Mail class="mr-2 h-4 w-4" />
						<span>Mail</span>
						<Command.Shortcut>⌘M</Command.Shortcut>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Files">
					<Command.Item>
						<FileText class="mr-2 h-4 w-4" />
						<span>Documents</span>
					</Command.Item>
					<Command.Item>
						<Search class="mr-2 h-4 w-4" />
						<span>Search Files</span>
						<Command.Shortcut>⌘F</Command.Shortcut>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Contacts">
					<Command.Item>
						<User class="mr-2 h-4 w-4" />
						<span>John Doe</span>
					</Command.Item>
					<Command.Item>
						<Phone class="mr-2 h-4 w-4" />
						<span>Call Contact</span>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Settings">
					<Command.Item>
						<Settings class="mr-2 h-4 w-4" />
						<span>Preferences</span>
						<Command.Shortcut>⌘,</Command.Shortcut>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="With Icons">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.Input placeholder="Search" />
			<Command.List>
				<Command.Empty>No results found.</Command.Empty>
				<Command.Group heading="Quick Actions">
					<Command.Item>
						<Home class="mr-2 h-4 w-4 text-blue-500" />
						<span>Go Home</span>
						<Command.Shortcut>⌘H</Command.Shortcut>
					</Command.Item>
					<Command.Item>
						<Search class="mr-2 h-4 w-4 text-green-500" />
						<span>Global Search</span>
						<Command.Shortcut>⌘/</Command.Shortcut>
					</Command.Item>
					<Command.Item>
						<Calculator class="mr-2 h-4 w-4 text-purple-500" />
						<span>Calculator</span>
						<Command.Shortcut>⌘K</Command.Shortcut>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Media">
					<Command.Item>
						<Image class="mr-2 h-4 w-4 text-pink-500" />
						<span>Photos</span>
					</Command.Item>
					<Command.Item>
						<Music class="mr-2 h-4 w-4 text-orange-500" />
						<span>Music Library</span>
					</Command.Item>
					<Command.Item>
						<Video class="mr-2 h-4 w-4 text-red-500" />
						<span>Videos</span>
					</Command.Item>
				</Command.Group>
				<Command.Separator />
				<Command.Group heading="Files & Folders">
					<Command.Item>
						<Folder class="mr-2 h-4 w-4 text-yellow-500" />
						<span>Documents</span>
					</Command.Item>
					<Command.Item>
						<FileText class="mr-2 h-4 w-4 text-gray-500" />
						<span>Recent Files</span>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="Empty">
	<StorybookContainer>
		<Command.Root
			value="xyz123nonexistent"
			class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]"
		>
			<Command.Input placeholder="Search" />
			<Command.List>
				<Command.Empty>
					<div class="py-6 text-center text-sm">
						<div class="mb-2">🔍</div>
						<div>No results found</div>
						<div class="text-muted-foreground mt-1 text-xs">
							Try a different search term
						</div>
					</div>
				</Command.Empty>
				<Command.Group heading="Suggestions">
					<Command.Item>
						<span>Calculator</span>
					</Command.Item>
					<Command.Item>
						<span>Calendar</span>
					</Command.Item>
					<Command.Item>
						<span>Settings</span>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="Loading">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.Input placeholder="Loading..." />
			<Command.List>
				<Command.Loading>
					<div class="flex items-center justify-center py-6">
						<div class="flex items-center space-x-2">
							<div
								class="h-4 w-4 animate-spin rounded-full border-2 border-gray-300 border-t-gray-600"
							></div>
							<span class="text-muted-foreground text-sm">Loading results...</span>
						</div>
					</div>
				</Command.Loading>
				<Command.Empty>No results found.</Command.Empty>
				<Command.Group heading="Recent">
					<Command.Item>
						<Search class="mr-2 h-4 w-4" />
						<span>Previous search</span>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</StorybookContainer>
</Story>

{#snippet template(args: { value: string }, _context: StoryContext<typeof Story>)}
	<StorybookContainer>
		<Command.Root
			bind:value={args.value}
			class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]"
		>
			<Command.Input placeholder="Type to search..." />
			<Command.List>
				<Command.Empty>No results found for "{args.value}"</Command.Empty>
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
{/snippet}
