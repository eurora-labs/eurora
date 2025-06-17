<script module lang="ts">
	import { ConversationItem } from '$lib/custom-components/conversation-item/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / ConversationItem',
		component: ConversationItem,
		parameters: {
			docs: {
				description: {
					component:
						'A comprehensive showcase of conversation item variants and states. Conversation items are used to display messages in a chat interface, supporting both agent and user messages with different visual styles.',
				},
			},
			controls: { disable: true },
		},
	});
</script>

<script lang="ts">
	import StoryContainer from '../StoryContainer.svelte';
</script>

<Story name="All Conversation Items">
	<StoryContainer class="p-0">
		{#snippet children()}
			<div class="backdrop-blur-2xl">
				<div class="space-y-8 p-6">
					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Basic User Message</h2>
						<ConversationItem variant="default" finishRendering={() => {}}>
							What's a fun fact I didn't know?
						</ConversationItem>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Basic Agent Message</h2>
						<ConversationItem variant="agent" finishRendering={() => {}}>
							I'm here to assist you with any questions you might have. What would you
							like to know?
						</ConversationItem>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Short Messages</h2>
						<div class="space-y-4">
							<ConversationItem variant="default" finishRendering={() => {}}>
								Hi!
							</ConversationItem>
							<ConversationItem variant="agent" finishRendering={() => {}}>
								Hello there!
							</ConversationItem>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">
							Medium Length Messages
						</h2>
						<div class="space-y-4">
							<ConversationItem variant="default" finishRendering={() => {}}>
								Can you help me understand how to implement a Svelte component with
								proper TypeScript types?
							</ConversationItem>
							<ConversationItem variant="agent" finishRendering={() => {}}>
								Absolutely! I'd be happy to help you with Svelte components and
								TypeScript. Let me walk you through the process step by step.
							</ConversationItem>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Long Messages</h2>
						<div class="space-y-4">
							<ConversationItem variant="default" finishRendering={() => {}}>
								I'm working on a complex project that involves multiple components,
								state management, and API integration. I need to understand the best
								practices for organizing my code, handling asynchronous operations,
								and ensuring type safety throughout the application. Could you
								provide some guidance on these topics?
							</ConversationItem>
							<ConversationItem variant="agent" finishRendering={() => {}}>
								That's a great question! When working on complex projects,
								organization and architecture are crucial. Here are some key
								principles to follow: First, establish a clear folder structure that
								separates concerns - components, stores, utilities, and types should
								have their own directories. For state management, consider using
								Svelte stores for global state and component-level state for local
								concerns. When handling async operations, use proper error handling
								with try-catch blocks and consider implementing loading states. For
								type safety, define interfaces for your data structures and use
								TypeScript's strict mode to catch potential issues early.
							</ConversationItem>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Code Examples</h2>
						<div class="space-y-4">
							<ConversationItem variant="default" finishRendering={() => {}}>
								Can you show me how to create a reactive store in Svelte?
							</ConversationItem>
							<ConversationItem variant="agent" finishRendering={() => {}}>
								{`
								Here's how you can create a reactive store in Svelte: 
								import {writable} from 'svelte/store'; 
								export const count = writable(0);
							`}
							</ConversationItem>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">
							Conversation Flow Example
						</h2>
						<div class="space-y-4">
							<ConversationItem variant="default" finishRendering={() => {}}>
								What's the weather like today?
							</ConversationItem>
							<ConversationItem variant="agent" finishRendering={() => {}}>
								I don't have access to real-time weather data, but I can help you
								find weather information. You could check a weather app or website
								like Weather.com or AccuWeather for current conditions in your area.
							</ConversationItem>
							<ConversationItem variant="default" finishRendering={() => {}}>
								Thanks! Can you help me with something else instead?
							</ConversationItem>
							<ConversationItem variant="agent" finishRendering={() => {}}>
								Of course! I'm here to help with a wide variety of topics. What
								would you like assistance with?
							</ConversationItem>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">With Custom Classes</h2>
						<div class="space-y-4">
							<ConversationItem
								variant="default"
								class=" text-white"
								finishRendering={() => {}}
							>
								This message has custom styling applied
							</ConversationItem>
							<ConversationItem
								variant="agent"
								class="text-white border-blue-500"
								finishRendering={() => {}}
							>
								This agent message also has custom styling
							</ConversationItem>
						</div>
					</div>
				</div>
			</div>
		{/snippet}
	</StoryContainer>
</Story>
