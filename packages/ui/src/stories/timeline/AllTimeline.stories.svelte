<script module lang="ts">
	import { Root } from '$lib/custom-components/timeline/index.js';
	import * as Timeline from '$lib/custom-components/timeline/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / Timeline',
		component: Root,
		parameters: {
			docs: {
				description: {
					component: 'A showcase of process timeline component',
				},
			},
			controls: { disable: true },
		},
	});
</script>

<script lang="ts">
	import { StorybookContainer } from '$lib/custom-components/storybook-container/index.js';
</script>

<Story name="Basic Timeline">
	{#snippet children()}
		<div class="h-[600px] w-full">
			<div class="h-full p-6 w-screen">
				<Timeline.Root class="p-6">
					<Timeline.Item color="yellow" name="GitHub" />
					<Timeline.Item color="red" name="StackOverflow" />
					<Timeline.Item color="blue" name="Reddit" />
					<Timeline.Item name="Slack" />
					<Timeline.Item name="Blender" />
				</Timeline.Root>
			</div>
		</div>
	{/snippet}
</Story>
