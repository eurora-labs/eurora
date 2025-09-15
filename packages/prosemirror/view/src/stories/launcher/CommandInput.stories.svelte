<script module lang="ts">
	import { CommandInput } from '$lib/launcher/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components/Launcher/CommandInput',
		component: CommandInput,
		parameters: {
			layout: 'centered',
			docs: {
				description: {
					component:
						'A specialized input component for command palettes with search icon and ProseMirror editor integration.',
				},
			},
		},
		argTypes: {
			value: {
				control: 'text',
				description: 'The input value',
			},
			placeholder: {
				control: 'text',
				description: 'Placeholder text',
			},
		},
		args: {
			value: '',
			placeholder: 'Search',
		},
	});
</script>

<script lang="ts">
	import * as Command from '$lib/launcher/index.js';
	import { StorybookContainer } from '@eurora/ui/storybook-container/index.js';
</script>

<Story name="Default">
	<StorybookContainer>
		<Command.Root class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]">
			<Command.Input placeholder="Search" />
		</Command.Root>
	</StorybookContainer>
</Story>

<Story name="With Value">
	<StorybookContainer>
		<Command.Root
			value="calculator"
			class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]"
		>
			<Command.Input placeholder="Search" />
		</Command.Root>
	</StorybookContainer>
</Story>

{#snippet template(
	args: { value: string; placeholder: string },
	_context: StoryContext<typeof Story>,
)}
	<StorybookContainer>
		<Command.Root
			bind:value={args.value}
			class="rounded-lg border bg-white/20 shadow-md backdrop-blur-[36px]"
		>
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
	</StorybookContainer>
{/snippet}
