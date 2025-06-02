<script module lang="ts">
	import { CommandGroup } from '$lib/custom-components/launcher/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components/Launcher/CommandGroup',
		component: CommandGroup,
		parameters: {
			layout: 'centered',
			docs: {
				description: {
					component: 'Groups related command items together with an optional heading.'
				}
			}
		},
		argTypes: {
			heading: {
				control: 'text',
				description: 'The heading text for the group'
			}
		},
		args: {
			heading: 'Applications'
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
		Folder,
		Music,
		Video,
		Image
	} from '@lucide/svelte';
</script>

<!-- Basic Group -->
<Story name="Basic Group">
	<div class="relative min-h-[300px] w-[900px] overflow-hidden rounded-lg">
		<div
			class="absolute inset-0 bg-cover bg-center bg-no-repeat"
			style="background-image: url('sample_background.jpg')"
		></div>
		<div class="relative z-10 flex min-h-full items-center justify-center p-6">
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
		</div>
	</div>
</Story>

<!-- Group without Heading -->
<Story name="Without Heading">
	<div class="relative min-h-[300px] w-[900px] overflow-hidden rounded-lg">
		<div
			class="absolute inset-0 bg-cover bg-center bg-no-repeat"
			style="background-image: url('sample_background.jpg')"
		></div>
		<div class="relative z-10 flex min-h-full items-center justify-center p-6">
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
		</div>
	</div>
</Story>

<!-- Multiple Groups -->
<Story name="Multiple Groups">
	<div class="relative min-h-[400px] w-[900px] overflow-hidden rounded-lg">
		<div
			class="absolute inset-0 bg-cover bg-center bg-no-repeat"
			style="background-image: url('sample_background.jpg')"
		></div>
		<div class="relative z-10 flex min-h-full items-center justify-center p-6">
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
		</div>
	</div>
</Story>

<!-- Groups with Shortcuts -->
<Story name="With Shortcuts">
	<div class="relative min-h-[300px] w-[900px] overflow-hidden rounded-lg">
		<div
			class="absolute inset-0 bg-cover bg-center bg-no-repeat"
			style="background-image: url('sample_background.jpg')"
		></div>
		<div class="relative z-10 flex min-h-full items-center justify-center p-6">
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
		</div>
	</div>
</Story>

<!-- Different Group Styles -->
<Story name="Different Headings">
	<div class="relative min-h-[300px] w-[900px] overflow-hidden rounded-lg">
		<div
			class="absolute inset-0 bg-cover bg-center bg-no-repeat"
			style="background-image: url('sample_background.jpg')"
		></div>
		<div class="relative z-10 flex min-h-full items-center justify-center p-6">
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
		</div>
	</div>
</Story>

<!-- Interactive Example -->
{#snippet template({ ...args }: Args<typeof Story>, _context: StoryContext<typeof Story>)}
	<div class="relative min-h-[300px] w-[900px] overflow-hidden rounded-lg">
		<div
			class="absolute inset-0 bg-cover bg-center bg-no-repeat"
			style="background-image: url('sample_background.jpg')"
		></div>
		<div class="relative z-10 flex min-h-full items-center justify-center p-6">
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
		</div>
	</div>
{/snippet}
