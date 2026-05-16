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
					component:
						'A vertical activity timeline. The first item (top) is the most recent; older items extend downward.',
				},
			},
			controls: { disable: true },
		},
	});
</script>

<Story name="Basic Timeline">
	{#snippet children()}
		<div class="h-[600px] w-12 border-r p-2">
			<Timeline.Root>
				<Timeline.Item color="#22c55e" name="Blender" highlighted />
				<Timeline.Item color="#3b82f6" name="Slack" />
				<Timeline.Item color="#ef4444" name="Reddit" />
				<Timeline.Item color="#f59e0b" name="StackOverflow" />
				<Timeline.Item color="#a855f7" name="GitHub" />
			</Timeline.Root>
		</div>
	{/snippet}
</Story>
