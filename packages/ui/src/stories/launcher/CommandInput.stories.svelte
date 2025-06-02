<script module lang="ts">
	import { CommandInput } from '$lib/custom-components/launcher/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components/Launcher/CommandInput',
		component: CommandInput,
		parameters: {
			layout: 'centered',
			docs: {
				description: {
					component:
						'A specialized input component for command palettes with search icon and ProseMirror editor integration.'
				}
			}
		},
		argTypes: {
			value: {
				control: 'text',
				description: 'The input value'
			},
			placeholder: {
				control: 'text',
				description: 'Placeholder text'
			}
		},
		args: {
			value: '',
			placeholder: 'Search'
		}
	});
</script>

<script lang="ts">
	import * as Command from '$lib/custom-components/launcher/index.js';
</script>

<!-- Default Input -->
<Story name="Default">
	<div class="w-[450px]">
		<Command.Root class="rounded-lg border shadow-md">
			<Command.Input placeholder="Search" />
		</Command.Root>
	</div>
</Story>

<!-- With Value -->
<Story name="With Value">
	<div class="w-[450px]">
		<Command.Root value="calculator" class="rounded-lg border shadow-md">
			<Command.Input placeholder="Search" />
		</Command.Root>
	</div>
</Story>

<!-- Interactive Example -->
{#snippet template({ ...args }: Args<typeof Story>, _context: StoryContext<typeof Story>)}
	<div class="w-[450px]">
		<Command.Root bind:value={args.value} class="rounded-lg border shadow-md">
			<Command.Input placeholder={args.placeholder} />
			<Command.List>
				<Command.Empty>No results found for "{args.value}"</Command.Empty>
				<Command.Group heading="Results">
					<Command.Item>
						<span>Sample result for: {args.value || 'empty search'}</span>
					</Command.Item>
				</Command.Group>
			</Command.List>
		</Command.Root>
	</div>
{/snippet}
