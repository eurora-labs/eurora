<script module lang="ts">
	import * as Timeline from '$lib/custom-components/timeline/index.js';
	import { Root } from '$lib/custom-components/timeline/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { SiGithub, SiStackoverflow, SiReddit } from '@icons-pack/svelte-simple-icons';

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
					<Timeline.Item color="yellow"><SiGithub /></Timeline.Item>
					<Timeline.Item color="red"><SiStackoverflow /></Timeline.Item>
					<Timeline.Item color="blue"><SiReddit /></Timeline.Item>
					<Timeline.Item>Slack</Timeline.Item>
					<Timeline.Item>Blender</Timeline.Item>
				</Timeline.Root>
			</div>
		</div>
	{/snippet}
</Story>
