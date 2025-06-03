<script module lang="ts">
	import { Button } from '$lib/components/button/index.js';
	import { defineMeta, type StoryContext, type Args } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / Button',
		component: Button,
		parameters: {
			docs: {
				description: {
					component:
						'A versatile button component with multiple variants and sizes. Can render as either a button or anchor element.',
				},
			},
		},
		argTypes: {
			variant: {
				control: { type: 'select' },
				options: ['default', 'destructive', 'outline', 'secondary', 'ghost', 'link'],
				description: 'The visual style variant of the button',
			},
			size: {
				control: { type: 'select' },
				options: ['default', 'sm', 'lg', 'icon'],
				description: 'The size of the button',
			},
			disabled: {
				control: { type: 'boolean' },
				description: 'Whether the button is disabled',
			},
			href: {
				control: { type: 'text' },
				description: 'If provided, renders as an anchor element instead of button',
			},
			type: {
				control: { type: 'select' },
				options: ['button', 'submit', 'reset'],
				description: 'The type attribute for button elements',
			},
		},
		args: {
			variant: 'default',
			size: 'default',
			disabled: false,
			type: 'button',
		},
	});
</script>

<!-- Interactive Button -->
{#snippet template({ ...args }: Args<typeof Story>, _context: StoryContext<typeof Story>)}
	<Button
		variant={args.variant}
		size={args.size}
		disabled={args.disabled}
		href={args.href}
		type={args.type}
	>
		{args.disabled ? 'Disabled' : 'Click me'}
	</Button>
{/snippet}

<Story name="Interactive" children={template} />
