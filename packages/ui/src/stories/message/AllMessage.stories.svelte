<script module lang="ts">
	import * as Message from '$lib/custom-components/message/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { SiGithub } from '@icons-pack/svelte-simple-icons';

	const { Story } = defineMeta({
		title: 'Components / Message',
		component: Message.Root,
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

<Story name="Basic Conversation Items">
	<StoryContainer class="p-0">
		{#snippet children()}
			<div>
				<div class="space-y-8 p-6">
					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Basic User Message</h2>
						<Message.Root variant="default" finishRendering={() => {}}>
							<Message.Content>What's a fun fact I didn't know?</Message.Content>
						</Message.Root>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Basic Agent Message</h2>
						<Message.Root variant="agent" finishRendering={() => {}}>
							<Message.Content>
								I'm here to assist you with any questions you might have. What would
								you like to know?
							</Message.Content>
						</Message.Root>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Short Messages</h2>
						<div class="space-y-4">
							<Message.Root variant="default" finishRendering={() => {}}>
								<Message.Content>Hi!</Message.Content>
							</Message.Root>
							<Message.Root variant="agent" finishRendering={() => {}}>
								<Message.Content>Hello there!</Message.Content>
							</Message.Root>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">
							Medium Length Messages
						</h2>
						<div class="space-y-4">
							<Message.Root variant="default" finishRendering={() => {}}>
								<Message.Content>
									Can you help me understand how to implement a Svelte component
									with proper TypeScript types?
								</Message.Content>
							</Message.Root>
							<Message.Root variant="agent" finishRendering={() => {}}>
								<Message.Content>
									Absolutely! I'd be happy to help you with Svelte components and
									TypeScript. Let me walk you through the process step by step.
								</Message.Content>
							</Message.Root>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Long Messages</h2>
						<div class="space-y-4">
							<Message.Root variant="default" finishRendering={() => {}}>
								<Message.Content>
									I'm working on a complex project that involves multiple
									components, state management, and API integration. I need to
									understand the best practices for organizing my code, handling
									asynchronous operations, and ensuring type safety throughout the
									application. Could you provide some guidance on these topics?
								</Message.Content>
							</Message.Root>
							<Message.Root variant="agent" finishRendering={() => {}}>
								<Message.Content>
									That's a great question! When working on complex projects,
									organization and architecture are crucial. Here are some key
									principles to follow: First, establish a clear folder structure
									that separates concerns - components, stores, utilities, and
									types should have their own directories. For state management,
									consider using Svelte stores for global state and
									component-level state for local concerns. When handling async
									operations, use proper error handling with try-catch blocks and
									consider implementing loading states. For type safety, define
									interfaces for your data structures and use TypeScript's strict
									mode to catch potential issues early.
								</Message.Content>
							</Message.Root>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">Code Examples</h2>
						<div class="space-y-4">
							<Message.Root variant="default" finishRendering={() => {}}>
								<Message.Content>
									Can you show me how to create a reactive store in Svelte?
								</Message.Content>
							</Message.Root>
							<Message.Root variant="agent" finishRendering={() => {}}>
								<Message.Content>
									{`
									Here's how you can create a reactive store in Svelte: 
									import {writable} from 'svelte/store'; 
									export const count = writable(0);
								`}
								</Message.Content>
							</Message.Root>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">
							Conversation Flow Example
						</h2>
						<div class="space-y-4">
							<Message.Root variant="default" finishRendering={() => {}}>
								<Message.Content>What's the weather like today?</Message.Content>
							</Message.Root>
							<Message.Root variant="agent" finishRendering={() => {}}>
								<Message.Content>
									I don't have access to real-time weather data, but I can help
									you find weather information. You could check a weather app or
									website like Weather.com or AccuWeather for current conditions
									in your area.
								</Message.Content>
							</Message.Root>
							<Message.Root variant="default" finishRendering={() => {}}>
								<Message.Content>
									Thanks! Can you help me with something else instead?
								</Message.Content>
							</Message.Root>
							<Message.Root variant="agent" finishRendering={() => {}}>
								<Message.Content>
									Of course! I'm here to help with a wide variety of topics. What
									would you like assistance with?
								</Message.Content>
							</Message.Root>
						</div>
					</div>

					<div>
						<h2 class="text-lg font-semibold mb-4 text-white">With Custom Classes</h2>
						<div class="space-y-4">
							<Message.Root
								variant="default"
								class=" text-white"
								finishRendering={() => {}}
							>
								<Message.Content>
									This message has custom styling applied
								</Message.Content>
							</Message.Root>
							<Message.Root
								variant="agent"
								class="text-white border-blue-500"
								finishRendering={() => {}}
							>
								<Message.Content>
									This agent message also has custom styling
								</Message.Content>
							</Message.Root>
						</div>
					</div>
				</div>
			</div>
		{/snippet}
	</StoryContainer>
</Story>

<Story name="Conversation Items With Sources">
	<StoryContainer class="p-0">
		{#snippet children()}
			<div class="h-screen">
				<div class="space-y-8 p-6">
					<div class="space-y-4">
						<h2 class="text-lg font-semibold mb-4 text-white">Answer with Source</h2>
						<Message.Root variant="default" finishRendering={() => {}}>
							<Message.Content>What's a fun fact I didn't know?</Message.Content>
						</Message.Root>
						<Message.Root variant="agent" finishRendering={() => {}}>
							<Message.Content>
								I'm here to assist you with any questions you might have. What would
								you like to know?
							</Message.Content>
							<Message.Footer>
								<Message.Source><SiGithub /></Message.Source>
							</Message.Footer>
						</Message.Root>
					</div>
				</div>
			</div>
		{/snippet}
	</StoryContainer>
</Story>
