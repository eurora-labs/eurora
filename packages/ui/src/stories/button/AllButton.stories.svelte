<script module lang="ts">
	import { Button } from '$lib/components/button/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / Button',
		component: Button,
		parameters: {
			docs: {
				description: {
					component:
						'A comprehensive showcase of all button variants, sizes, and states. This page displays all button examples without interactive controls.',
				},
			},
			controls: { disable: true },
		},
	});
</script>

<script lang="ts">
	import { ChevronRight, Download, Heart, Plus, Settings, Trash2 } from '@lucide/svelte';
</script>

<Story name="All Buttons">
	<div class="space-y-8 p-6">
		<div>
			<h2 class="text-lg font-semibold mb-4">Default Button</h2>
			<Button>Click me</Button>
		</div>

		<div>
			<h2 class="text-lg font-semibold mb-4">All Variants</h2>
			<div class="flex flex-wrap gap-4">
				<Button variant="default">Default</Button>
				<Button variant="destructive">Destructive</Button>
				<Button variant="outline">Outline</Button>
				<Button variant="secondary">Secondary</Button>
				<Button variant="ghost">Ghost</Button>
				<Button variant="link">Link</Button>
			</div>
		</div>

		<div>
			<h2 class="text-lg font-semibold mb-4">All Sizes</h2>
			<div class="flex flex-wrap items-center gap-4">
				<Button size="sm">Small</Button>
				<Button size="default">Default</Button>
				<Button size="lg">Large</Button>
				<Button size="icon">
					{#snippet children()}<Settings class="h-4 w-4" />{/snippet}
				</Button>
			</div>
		</div>

		<div>
			<h2 class="text-lg font-semibold mb-4">With Icons</h2>
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
		</div>

		<div>
			<h2 class="text-lg font-semibold mb-4">Icon Only Buttons</h2>
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
		</div>

		<div>
			<h2 class="text-lg font-semibold mb-4">Disabled States</h2>
			<div class="flex flex-wrap gap-4">
				<Button disabled>Default Disabled</Button>
				<Button variant="destructive" disabled>Destructive Disabled</Button>
				<Button variant="outline" disabled>Outline Disabled</Button>
				<Button variant="secondary" disabled>Secondary Disabled</Button>
				<Button variant="ghost" disabled>Ghost Disabled</Button>
				<Button variant="link" disabled>Link Disabled</Button>
			</div>
		</div>

		<div>
			<h2 class="text-lg font-semibold mb-4">As Links</h2>
			<div class="flex flex-wrap gap-4">
				<Button href="https://example.com">External Link</Button>
				<Button href="/internal" variant="outline">Internal Link</Button>
				<Button href="https://example.com" variant="link">Link Style</Button>
				<Button href="https://example.com" disabled>Disabled Link</Button>
			</div>
		</div>

		<div>
			<h2 class="text-lg font-semibold mb-4">Loading States</h2>
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
		</div>
	</div>
</Story>
