<script module lang="ts">
	import { ContextChip } from '$lib/custom-components/context-chip/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / ContextChip',
		component: ContextChip,
		parameters: {
			docs: {
				description: {
					component:
						'A comprehensive showcase of all context chip variants and states. Context chips are used to display contextual information with backdrop blur effects and various visual styles.',
				},
			},
			controls: { disable: true },
		},
	});
</script>

<script lang="ts">
	import StoryContainer from '../StoryContainer.svelte';
	import { ExternalLink, Hash, Tag, User, Calendar, MapPin, Clock } from '@lucide/svelte';
</script>

<Story name="All Context Chips">
	<StoryContainer>
		{#snippet children()}
			<div class="space-y-8 p-6">
				<div>
					<h2 class="text-lg font-semibold mb-4 text-white">Default Context Chip</h2>
					<ContextChip>Default Chip</ContextChip>
				</div>

				<div>
					<h2 class="text-lg font-semibold mb-4 text-white">All Variants</h2>
					<div class="flex flex-wrap gap-4">
						<ContextChip variant="default">Default</ContextChip>
						<ContextChip variant="primary">Primary</ContextChip>
						<ContextChip variant="secondary">Secondary</ContextChip>
						<ContextChip variant="destructive">Destructive</ContextChip>
						<ContextChip variant="outline">Outline</ContextChip>
					</div>
				</div>

				<div>
					<h2 class="text-lg font-semibold mb-4 text-white">With Icons</h2>
					<div class="flex flex-wrap gap-4">
						<ContextChip variant="default">
							<Hash class="mr-2 h-4 w-4" />
							Tag
						</ContextChip>
						<ContextChip variant="primary">
							<User class="mr-2 h-4 w-4" />
							User
						</ContextChip>
						<ContextChip variant="secondary">
							<Calendar class="mr-2 h-4 w-4" />
							Event
						</ContextChip>
						<ContextChip variant="outline">
							<MapPin class="mr-2 h-4 w-4" />
							Location
						</ContextChip>
					</div>
				</div>

				<div>
					<h2 class="text-lg font-semibold mb-4 text-white">Different Content Types</h2>
					<div class="flex flex-wrap gap-4">
						<ContextChip>Short</ContextChip>
						<ContextChip variant="primary">Medium Length Text</ContextChip>
						<ContextChip variant="secondary"
							>This is a longer context chip with more text</ContextChip
						>
						<ContextChip variant="outline">
							<Clock class="mr-2 h-4 w-4" />
							2024-06-03 08:50
						</ContextChip>
					</div>
				</div>

				<div>
					<h2 class="text-lg font-semibold mb-4 text-white">As Links</h2>
					<div class="flex flex-wrap gap-4">
						<ContextChip href="https://example.com" variant="default">
							<ExternalLink class="mr-2 h-4 w-4" />
							External Link
						</ContextChip>
						<ContextChip href="/internal" variant="primary">Internal Link</ContextChip>
						<ContextChip href="https://example.com" variant="outline">
							<Tag class="mr-2 h-4 w-4" />
							Tagged Link
						</ContextChip>
					</div>
				</div>

				<div>
					<h2 class="text-lg font-semibold mb-4 text-white">Interactive Chips</h2>
					<div class="flex flex-wrap gap-4">
						<ContextChip variant="default" onClick={() => alert('Default clicked!')}>
							Clickable Default
						</ContextChip>
						<ContextChip variant="primary" onClick={() => alert('Primary clicked!')}>
							<User class="mr-2 h-4 w-4" />
							Clickable Primary
						</ContextChip>
						<ContextChip
							variant="destructive"
							onClick={() => alert('Destructive clicked!')}
						>
							Remove Tag
						</ContextChip>
					</div>
				</div>

				<div>
					<h2 class="text-lg font-semibold mb-4 text-white">Mixed Usage Examples</h2>
					<div class="space-y-4">
						<div class="flex flex-wrap gap-2">
							<span class="text-white text-sm">Article tags:</span>
							<ContextChip variant="outline">JavaScript</ContextChip>
							<ContextChip variant="outline">React</ContextChip>
							<ContextChip variant="outline">TypeScript</ContextChip>
						</div>
						<div class="flex flex-wrap gap-2">
							<span class="text-white text-sm">User info:</span>
							<ContextChip variant="primary">
								<User class="mr-2 h-4 w-4" />
								John Doe
							</ContextChip>
							<ContextChip variant="secondary">
								<Calendar class="mr-2 h-4 w-4" />
								Joined 2024
							</ContextChip>
						</div>
						<div class="flex flex-wrap gap-2">
							<span class="text-white text-sm">Status indicators:</span>
							<ContextChip variant="default">Active</ContextChip>
							<ContextChip variant="destructive">Expired</ContextChip>
							<ContextChip variant="primary">Premium</ContextChip>
						</div>
					</div>
				</div>
			</div>
		{/snippet}
	</StoryContainer>
</Story>
