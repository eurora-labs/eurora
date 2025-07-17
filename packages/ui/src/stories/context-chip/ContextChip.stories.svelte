<script module lang="ts">
	import { ContextChip } from '$lib/custom-components/context-chip/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / ContextChip',
		component: ContextChip,
		parameters: {
			docs: {
				description: {
					component:
						'A versatile context chip component with backdrop blur effects and multiple variants. Can render as either a span, button, or anchor element.',
				},
			},
		},
		argTypes: {
			variant: {
				control: { type: 'select' },
				options: ['default', 'primary', 'secondary', 'destructive', 'outline'],
				description: 'The visual style variant of the context chip',
			},
			href: {
				control: { type: 'text' },
				description: 'If provided, renders as an anchor element instead of span',
			},
			onclick: {
				control: { type: 'boolean' },
				description: 'Whether the chip has click functionality (renders as button role)',
			},
			class: {
				control: { type: 'text' },
				description: 'Additional CSS classes to apply',
			},
		},
		args: {
			variant: 'default',
			href: undefined,
			onclick: undefined,
			class: undefined,
		},
	});
</script>

<script lang="ts">
	import { StorybookContainer } from '$lib/custom-components/storybook-container/index.js';
	import { Hash } from '@lucide/svelte';
</script>

<!-- Interactive Context Chip -->
{#snippet template(
	args: {
		variant: 'default' | 'primary' | 'secondary' | 'destructive' | 'outline' | undefined;
		href: string | undefined;
		onclick: boolean;
		class: string | undefined;
	},
	_context: StoryContext<typeof Story>,
)}
	<StorybookContainer>
		<ContextChip
			variant={args.variant}
			href={args.href}
			onclick={args.onclick ? () => alert('Context chip clicked!') : undefined}
			class={args.class}
		>
			{#if args.href}
				<Hash class="mr-2 h-4 w-4" />
				Link Chip
			{:else if args.onclick}
				<Hash class="mr-2 h-4 w-4" />
				Clickable Chip
			{:else}
				<Hash class="mr-2 h-4 w-4" />
				Context Chip
			{/if}
		</ContextChip>
	</StorybookContainer>
{/snippet}
