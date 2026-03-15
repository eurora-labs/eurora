<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { Root as ChainOfThoughtRoot } from '$lib/components/ai-elements/chain-of-thought/index';
	import * as ChainOfThought from '$lib/components/ai-elements/chain-of-thought/index';

	const { Story } = defineMeta({
		title: 'AI Elements / Chain of Thought',
		component: ChainOfThoughtRoot,
		parameters: {
			layout: 'padded',
			controls: { disable: true },
			docs: {
				description: {
					component:
						'Chain of Thought component showing step-by-step reasoning with search results, images, and status indicators.',
				},
			},
		},
	});
</script>

<script lang="ts">
	import SearchIcon from '@lucide/svelte/icons/search';
	import ImageIcon from '@lucide/svelte/icons/image';

	const profileImageUrl = 'https://d26xptavrz5c8t.cloudfront.net/image/andre.png';

	const searchResults = [
		'https://www.linkedin.com/in/andre-roelofs/',
		'https://github.com/eurora-labs/eurora',
		'https://www.eurora-labs.com',
	];

	const recentWorkResults = [
		'https://github.com/eurora-labs/eurora',
		'https://www.eurora-labs.com',
	];
</script>

<Story name="Chain of Thought">
	<div class="w-125">
		<ChainOfThought.Root defaultOpen>
			<ChainOfThought.Header />
			<ChainOfThought.Content>
				<ChainOfThought.Step icon={SearchIcon} status="complete">
					{#snippet label()}
						Searching for profiles for Andre Roelofs
					{/snippet}
					<ChainOfThought.SearchResults>
						{#each searchResults as website}
							<ChainOfThought.SearchResult>
								{new URL(website).hostname}
							</ChainOfThought.SearchResult>
						{/each}
					</ChainOfThought.SearchResults>
				</ChainOfThought.Step>

				<ChainOfThought.Step icon={ImageIcon} status="complete">
					{#snippet label()}
						Found the profile photo for Andre Roelofs
					{/snippet}
					<ChainOfThought.Image
						caption="Andre Roelofs's profile photo from LinkedIn, showing a Dutch AI engineer."
					>
						<img
							src={profileImageUrl}
							alt="Andre Roelofs profile photo"
							class="aspect-square h-[150px] rounded border object-cover"
						/>
					</ChainOfThought.Image>
				</ChainOfThought.Step>

				<ChainOfThought.Step status="complete">
					{#snippet label()}
						Andre Roelofs is a Dutch AI engineer and founder of Eurora. He previously
						led engineering and ML at Cuebric, developing award-winning AI solutions for
						the entertainment industry.
					{/snippet}
				</ChainOfThought.Step>

				<ChainOfThought.Step icon={SearchIcon} status="active">
					{#snippet label()}
						Searching for recent work...
					{/snippet}
					<ChainOfThought.SearchResults>
						{#each recentWorkResults as website}
							<ChainOfThought.SearchResult>
								{new URL(website).hostname}
							</ChainOfThought.SearchResult>
						{/each}
					</ChainOfThought.SearchResults>
				</ChainOfThought.Step>
			</ChainOfThought.Content>
		</ChainOfThought.Root>
	</div>
</Story>
