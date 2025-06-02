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
						'A versatile button component with multiple variants and sizes. Can render as either a button or anchor element.'
				}
			}
		},
		argTypes: {
			variant: {
				control: { type: 'select' },
				options: ['default', 'destructive', 'outline', 'secondary', 'ghost', 'link'],
				description: 'The visual style variant of the button'
			},
			size: {
				control: { type: 'select' },
				options: ['default', 'sm', 'lg', 'icon'],
				description: 'The size of the button'
			},
			disabled: {
				control: { type: 'boolean' },
				description: 'Whether the button is disabled'
			},
			href: {
				control: { type: 'text' },
				description: 'If provided, renders as an anchor element instead of button'
			},
			type: {
				control: { type: 'select' },
				options: ['button', 'submit', 'reset'],
				description: 'The type attribute for button elements'
			}
		},
		args: {
			variant: 'default',
			size: 'default',
			disabled: false,
			type: 'button'
		}
	});
</script>

<script lang="ts">
	import { ChevronRight, Download, Heart, Plus, Settings, Trash2 } from '@lucide/svelte';
</script>

<!-- Default Button -->
<Story name="Default">
	<Button>Click me</Button>
</Story>

<!-- All Variants -->
<Story name="Variants">
	<div class="flex flex-wrap gap-4">
		<Button variant="default">Default</Button>
		<Button variant="destructive">Destructive</Button>
		<Button variant="outline">Outline</Button>
		<Button variant="secondary">Secondary</Button>
		<Button variant="ghost">Ghost</Button>
		<Button variant="link">Link</Button>
	</div>
</Story>

<!-- All Sizes -->
<Story name="Sizes">
	<div class="flex flex-wrap items-center gap-4">
		<Button size="sm">Small</Button>
		<Button size="default">Default</Button>
		<Button size="lg">Large</Button>
		<Button size="icon">
			{#snippet children()}<Settings class="h-4 w-4" />{/snippet}
		</Button>
	</div>
</Story>

<!-- With Icons -->
<Story name="With Icons">
	<div class="flex flex-wrap gap-4">
		<Button>
			{#snippet children()}
				<Download class="mr-2 h-4 w-4" />
				Download
			{/snippet}
		</Button>
		<Button variant="outline">
			{#snippet children()}
				<Plus class="mr-2 h-4 w-4" />
				Add Item
			{/snippet}
		</Button>
		<Button variant="destructive">
			{#snippet children()}
				<Trash2 class="mr-2 h-4 w-4" />
				Delete
			{/snippet}
		</Button>
		<Button variant="ghost">
			{#snippet children()}
				Continue
				<ChevronRight class="ml-2 h-4 w-4" />
			{/snippet}
		</Button>
	</div>
</Story>

<!-- Icon Only Buttons -->
<Story name="Icon Only">
	<div class="flex flex-wrap gap-4">
		<Button size="icon">
			{#snippet children()}<Heart class="h-4 w-4" />{/snippet}
		</Button>
		<Button size="icon" variant="outline">
			{#snippet children()}<Settings class="h-4 w-4" />{/snippet}
		</Button>
		<Button size="icon" variant="destructive">
			{#snippet children()}<Trash2 class="h-4 w-4" />{/snippet}
		</Button>
		<Button size="icon" variant="ghost">
			{#snippet children()}<Plus class="h-4 w-4" />{/snippet}
		</Button>
	</div>
</Story>

<!-- Disabled States -->
<Story name="Disabled">
	<div class="flex flex-wrap gap-4">
		<Button disabled>Default Disabled</Button>
		<Button variant="destructive" disabled>Destructive Disabled</Button>
		<Button variant="outline" disabled>Outline Disabled</Button>
		<Button variant="secondary" disabled>Secondary Disabled</Button>
		<Button variant="ghost" disabled>Ghost Disabled</Button>
		<Button variant="link" disabled>Link Disabled</Button>
	</div>
</Story>

<!-- As Links -->
<Story name="As Links">
	<div class="flex flex-wrap gap-4">
		<Button href="https://example.com">External Link</Button>
		<Button href="/internal" variant="outline">Internal Link</Button>
		<Button href="https://example.com" variant="link">Link Style</Button>
		<Button href="https://example.com" disabled>Disabled Link</Button>
	</div>
</Story>

<!-- Loading States -->
<Story name="Loading">
	<div class="flex flex-wrap gap-4">
		<Button disabled>
			{#snippet children()}
				<div
					class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
				></div>
				Loading...
			{/snippet}
		</Button>
		<Button variant="outline" disabled>
			{#snippet children()}
				<div
					class="mr-2 h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
				></div>
				Processing
			{/snippet}
		</Button>
		<Button size="icon" disabled>
			{#snippet children()}
				<div
					class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
				></div>
			{/snippet}
		</Button>
	</div>
</Story>
<!-- Interactive Example -->
{#snippet template({ ...args }: Args<typeof Story>, _context: StoryContext<typeof Story>)}
	<Button variant={args.variant} size={args.size} disabled={args.disabled}>
		{args.disabled ? 'Disabled' : 'Click me'}
	</Button>
{/snippet}
