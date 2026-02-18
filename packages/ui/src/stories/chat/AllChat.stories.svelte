<script module lang="ts">
	import * as Chat from '$lib/custom-components/chat/index.js';
	import { Root } from '$lib/custom-components/chat/index.js';
	import { SiGithub, SiStackoverflow, SiReddit } from '@icons-pack/svelte-simple-icons';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / Chat',
		component: Root,
		parameters: {
			docs: {
				description: {
					component:
						'A comprehensive showcase of chat interface variants demonstrating scrolling behavior, different chat lengths, presence of sources, and interactive features. The chat component serves as a container for message components, supporting various thread scenarios.',
				},
			},
			controls: { disable: true },
		},
	});

	// Helper function to render icons as HTML strings
	function renderIcon(IconComponent: any): string {
		// This is a simplified approach - in a real implementation you'd want proper icon rendering
		if (IconComponent === SiGithub)
			return '<svg class="size-4" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>';
		if (IconComponent === SiStackoverflow)
			return '<svg class="size-4" viewBox="0 0 24 24" fill="currentColor"><path d="M15.725 0l-1.72 1.277 6.39 8.588 1.716-1.277L15.725 0zm-3.94 3.418l-1.369 1.644 8.225 6.85 1.369-1.644-8.225-6.85zm-3.15 4.465l-.905 1.94 9.702 4.517.904-1.94-9.701-4.517zm-1.85 4.86l-.44 2.093 10.473 2.201.44-2.092-10.473-2.203zM1.89 15.47V24h19.19v-8.53h-2.133v6.397H4.021v-6.396H1.89zm4.265 2.133v2.13h10.66v-2.13H6.154Z"/></svg>';
		if (IconComponent === SiReddit)
			return '<svg class="size-4" viewBox="0 0 24 24" fill="currentColor"><path d="M12 0A12 12 0 0 0 0 12a12 12 0 0 0 12 12 12 12 0 0 0 12-12A12 12 0 0 0 12 0zm5.01 4.744c.688 0 1.25.561 1.25 1.249a1.25 1.25 0 0 1-2.498.056l-2.597-.547-.8 3.747c1.824.07 3.48.632 4.674 1.488.308-.309.73-.491 1.207-.491.968 0 1.754.786 1.754 1.754 0 .716-.435 1.333-1.01 1.614a3.111 3.111 0 0 1 .042.52c0 2.694-3.13 4.87-7.004 4.87-3.874 0-7.004-2.176-7.004-4.87 0-.183.015-.366.043-.534A1.748 1.748 0 0 1 4.028 12c0-.968.786-1.754 1.754-1.754.463 0 .898.196 1.207.49 1.207-.883 2.878-1.43 4.744-1.487l.885-4.182a.342.342 0 0 1 .14-.197.35.35 0 0 1 .238-.042l2.906.617a1.214 1.214 0 0 1 1.108-.701zM9.25 12C8.561 12 8 12.562 8 13.25c0 .687.561 1.248 1.25 1.248.687 0 1.248-.561 1.248-1.249 0-.688-.561-1.249-1.249-1.249zm5.5 0c-.687 0-1.248.561-1.248 1.25 0 .687.561 1.248 1.249 1.248.688 0 1.249-.561 1.249-1.249 0-.687-.562-1.249-1.25-1.249zm-5.466 3.99a.327.327 0 0 0-.231.094.33.33 0 0 0 0 .463c.842.842 2.484.913 2.961.913.477 0 2.105-.056 2.961-.913a.361.361 0 0 0 .029-.463.33.33 0 0 0-.464 0c-.547.533-1.684.73-2.512.73-.828 0-1.979-.196-2.512-.73a.326.326 0 0 0-.232-.095z"/></svg>';
		return '';
	}

	// Sample message data for different scenarios
	const shortThreadMessages = [
		{
			role: 'user' as const,
			content: 'Hi! Can you help me with Svelte?',
		},
		{
			role: 'system' as const,
			content:
				"Of course! I'd be happy to help you with Svelte. What specific aspect would you like to learn about?",
		},
		{
			role: 'user' as const,
			content: 'How do I create reactive variables?',
		},
		{
			role: 'system' as const,
			content:
				"In Svelte, you can create reactive variables using `$state()` for component state or `$derived()` for computed values. Here's a simple example: `let count = $state(0);`",
			sources: [renderIcon(SiGithub)],
		},
	];

	const mediumThreadMessages = [
		{
			role: 'user' as const,
			content:
				"I'm building a web application with Svelte. Can you guide me through the process?",
		},
		{
			role: 'system' as const,
			content:
				"Absolutely! Building a web application with Svelte is a great choice. Let's start with the basics. What type of application are you planning to build?",
		},
		{
			role: 'user' as const,
			content: 'A task management app with user authentication and real-time updates.',
		},
		{
			role: 'system' as const,
			content:
				"Perfect! For a task management app, you'll need several key components: authentication, state management, real-time updates, and a clean UI. Let's break this down step by step.",
			sources: [renderIcon(SiStackoverflow)],
		},
		{
			role: 'user' as const,
			content: "What's the best way to handle authentication in Svelte?",
		},
		{
			role: 'system' as const,
			content:
				"For authentication in Svelte, you have several options. You can use SvelteKit's built-in authentication with adapters, or integrate with services like Auth0, Firebase Auth, or Supabase. The choice depends on your specific needs and infrastructure.",
			sources: [renderIcon(SiGithub), renderIcon(SiReddit)],
		},
		{
			role: 'user' as const,
			content: 'How about state management for the tasks?',
		},
		{
			role: 'system' as const,
			content:
				"For state management, Svelte's built-in stores are excellent for most use cases. You can use writable stores for task data, derived stores for computed values like filtered tasks, and readable stores for real-time data from your backend.",
		},
		{
			role: 'user' as const,
			content: 'Can you show me a basic example of a task store?',
		},
		{
			role: 'system' as const,
			content: `Here's a basic task store example:

import { writable, derived } from 'svelte/store';

export const tasks = writable([]);
export const completedTasks = derived(tasks, $tasks =>
  $tasks.filter(task => task.completed)
);

export function addTask(task) {
  tasks.update(currentTasks => [...currentTasks, task]);
}`,
			sources: [renderIcon(SiGithub)],
		},
	];

	const longThreadMessages = [
		{
			role: 'user' as const,
			content:
				'I need help building a complex e-commerce platform with Svelte. Where should I start?',
		},
		{
			role: 'system' as const,
			content:
				"Building an e-commerce platform is a substantial project! Let's break it down into manageable phases. We'll need to consider architecture, user management, product catalog, shopping cart, payment processing, and order management.",
			sources: [renderIcon(SiGithub)],
		},
		{
			role: 'user' as const,
			content: "What's the recommended project structure for a large Svelte application?",
		},
		{
			role: 'system' as const,
			content:
				'For large applications, I recommend organizing by feature rather than file type. Create directories like `src/features/auth`, `src/features/products`, `src/features/cart`, etc. Each feature should contain its components, stores, and utilities.',
			sources: [renderIcon(SiStackoverflow)],
		},
		{
			role: 'user' as const,
			content: 'How do I handle complex state management across multiple features?',
		},
		{
			role: 'system' as const,
			content:
				'For complex state management, consider using a combination of Svelte stores and context API. Create a global store for shared state like user authentication and cart data, while keeping feature-specific state local to each feature module.',
			sources: [renderIcon(SiReddit), renderIcon(SiGithub)],
		},
		{
			role: 'user' as const,
			content: 'What about handling API calls and data fetching?',
		},
		{
			role: 'system' as const,
			content:
				"For API management, create a dedicated service layer. Use SvelteKit's load functions for server-side data fetching, and implement client-side fetching with proper error handling and loading states. Consider using libraries like TanStack Query for advanced caching.",
		},
		{
			role: 'user' as const,
			content: 'How should I implement the shopping cart functionality?',
		},
		{
			role: 'system' as const,
			content:
				'The shopping cart should be a global store that persists across sessions. Implement add/remove/update quantity functions, calculate totals with tax and shipping, and sync with the backend for logged-in users while supporting guest checkout.',
			sources: [renderIcon(SiGithub)],
		},
		{
			role: 'user' as const,
			content: "What's the best approach for handling payments securely?",
		},
		{
			role: 'system' as const,
			content:
				'Never handle payment details directly in your frontend. Use payment processors like Stripe, PayPal, or Square. Implement their client-side SDKs to tokenize payment information, then send tokens to your secure backend for processing.',
			sources: [renderIcon(SiStackoverflow), renderIcon(SiReddit)],
		},
		{
			role: 'user' as const,
			content: 'How do I optimize performance for a large product catalog?',
		},
		{
			role: 'system' as const,
			content:
				'Implement virtual scrolling for large lists, lazy load images, use pagination or infinite scrolling, implement search with debouncing, and consider server-side filtering. Cache frequently accessed data and use CDN for static assets.',
		},
		{
			role: 'user' as const,
			content: 'What about SEO and social media integration?',
		},
		{
			role: 'system' as const,
			content:
				"Use SvelteKit's server-side rendering for SEO. Implement proper meta tags, structured data for products, Open Graph tags for social sharing, and ensure fast loading times. Consider implementing breadcrumbs and clean URLs.",
			sources: [renderIcon(SiGithub)],
		},
		{
			role: 'user' as const,
			content: 'How should I handle user authentication and authorization?',
		},
		{
			role: 'system' as const,
			content:
				"Implement JWT-based authentication with refresh tokens. Use SvelteKit's hooks for server-side auth checks, create protected routes, implement role-based access control for admin features, and ensure secure session management.",
			sources: [renderIcon(SiStackoverflow)],
		},
		{
			role: 'user' as const,
			content:
				'This is really helpful! Can you provide a basic code example for the cart store?',
		},
		{
			role: 'system' as const,
			content: `Here's a comprehensive cart store example:

import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

function createCartStore() {
  const { subscribe, set, update } = writable([]);

  return {
    subscribe,
    addItem: (product) => update(items => {
      const existing = items.find(item => item.id === product.id);
      if (existing) {
        existing.quantity += 1;
        return items;
      }
      return [...items, { ...product, quantity: 1 }];
    }),
    removeItem: (id) => update(items => items.filter(item => item.id !== id)),
    updateQuantity: (id, quantity) => update(items =>
      items.map(item => item.id === id ? { ...item, quantity } : item)
    ),
    clear: () => set([])
  };
}

export const cart = createCartStore();
export const cartTotal = derived(cart, $cart =>
  $cart.reduce((total, item) => total + (item.price * item.quantity), 0)
);`,
			sources: [renderIcon(SiGithub)],
		},
	];

	const mixedContentMessages = [
		{
			role: 'user' as const,
			content: 'I need information about modern web development practices.',
		},
		{
			role: 'system' as const,
			content:
				'Modern web development emphasizes performance, accessibility, and user experience. Key practices include component-based architecture, responsive design, progressive enhancement, and proper SEO implementation.',
			sources: [renderIcon(SiGithub)],
		},
		{
			role: 'user' as const,
			content: 'What are the current trends in frontend frameworks?',
		},
		{
			role: 'system' as const,
			content:
				"Current trends include server-side rendering with frameworks like Next.js and SvelteKit, component libraries, TypeScript adoption, and build tools like Vite. There's also growing interest in edge computing and JAMstack architecture.",
			sources: [renderIcon(SiStackoverflow), renderIcon(SiReddit)],
		},
		{
			role: 'user' as const,
			content: 'How important is TypeScript in modern development?',
		},
		{
			role: 'system' as const,
			content:
				'TypeScript has become essential for large-scale applications. It provides type safety, better IDE support, improved refactoring capabilities, and helps catch errors at compile time rather than runtime.',
		},
		{
			role: 'user' as const,
			content: 'What about testing strategies?',
		},
		{
			role: 'system' as const,
			content:
				'Modern testing includes unit tests with Vitest or Jest, component testing with Testing Library, end-to-end testing with Playwright or Cypress, and visual regression testing. The key is finding the right balance for your project.',
			sources: [renderIcon(SiGithub), renderIcon(SiStackoverflow)],
		},
		{
			role: 'user' as const,
			content: 'Can you recommend some performance optimization techniques?',
		},
		{
			role: 'system' as const,
			content:
				'Key performance optimizations include code splitting, lazy loading, image optimization, caching strategies, minimizing bundle sizes, using CDNs, implementing service workers, and optimizing Core Web Vitals metrics.',
			sources: [renderIcon(SiReddit)],
		},
	];

	const interactiveMessages = [
		{
			role: 'system' as const,
			content:
				"Welcome! I'm here to help you with any questions. This chat demonstrates auto-scrolling behavior and interactive features.",
		},
		{
			role: 'user' as const,
			content: 'How does the scrolling work in this chat interface?',
		},
		{
			role: 'system' as const,
			content:
				'The chat uses a ScrollArea component that automatically handles overflow. When new messages are added, the chat can be programmed to auto-scroll to the bottom, ensuring users always see the latest messages.',
			sources: [renderIcon(SiGithub)],
		},
		{
			role: 'user' as const,
			content: 'What about handling very long messages?',
		},
		{
			role: 'system' as const,
			content:
				"Long messages are handled gracefully with proper text wrapping and responsive design. The message containers adapt to content length while maintaining readability and proper spacing. Here's an example of a longer message that demonstrates how the layout adapts to accommodate more content without breaking the overall chat flow. The message bubbles expand vertically as needed while maintaining their maximum width constraints to ensure optimal reading experience across different screen sizes.",
		},
		{
			role: 'user' as const,
			content: 'Can messages include code blocks?',
		},
		{
			role: 'system' as const,
			content: `Yes! Messages can include formatted code blocks:

function handleScroll() {
  const chatContainer = document.getElementById('chat');
  const shouldAutoScroll = chatContainer.scrollTop + chatContainer.clientHeight >= chatContainer.scrollHeight - 10;

  if (shouldAutoScroll) {
    chatContainer.scrollTop = chatContainer.scrollHeight;
  }
}`,
			sources: [renderIcon(SiStackoverflow)],
		},
		{
			role: 'user' as const,
			content: 'How are sources displayed and managed?',
		},
		{
			role: 'system' as const,
			content:
				'Sources are displayed as icons in the message footer. They can represent different types of references like documentation, Stack Overflow answers, GitHub repositories, or other relevant resources. Multiple sources can be shown for a single message.',
			sources: [renderIcon(SiGithub), renderIcon(SiStackoverflow), renderIcon(SiReddit)],
		},
		{
			role: 'user' as const,
			content:
				'This looks great! The scrolling behavior is smooth and the layout is responsive.',
		},
		{
			role: 'system' as const,
			content:
				'Thank you! The chat interface is designed to provide a smooth user experience with proper message spacing, responsive design, and intuitive scrolling behavior. Feel free to explore the different story variants to see various chat scenarios.',
		},
	];
</script>

<script lang="ts">
	import { StorybookContainer } from '$lib/custom-components/storybook-container/index.js';
</script>

<Story name="Short Thread">
	<StorybookContainer class="p-0">
		{#snippet children()}
			<div class="h-[600px] w-full">
				<div class="h-full p-6">
					<h2 class="text-lg font-semibold mb-4 text-white">
						Short Thread (3-4 messages)
					</h2>
					<Chat.Root class="h-[500px]">
						{#each shortThreadMessages as message}
							<Chat.Message
								variant={message.role === 'user' ? 'default' : 'assistant'}
								finishRendering={() => {}}
							>
								<Chat.MessageContent>{message.content}</Chat.MessageContent>
								{#if message.sources && message.sources.length > 0}
									<Chat.MessageFooter>
										<Chat.MessageSource>
											{#each message.sources as source}
												{@html source}
											{/each}
										</Chat.MessageSource>
									</Chat.MessageFooter>
								{/if}
							</Chat.Message>
						{/each}
					</Chat.Root>
				</div>
			</div>
		{/snippet}
	</StorybookContainer>
</Story>

<Story name="Medium Thread">
	<StorybookContainer class="p-0">
		{#snippet children()}
			<div class="h-[600px] w-full">
				<div class="h-full p-6">
					<h2 class="text-lg font-semibold mb-4 text-white">
						Medium Thread (8-10 messages)
					</h2>
					<Chat.Root class="h-[500px]">
						{#each mediumThreadMessages as message}
							<Chat.Message
								variant={message.role === 'user' ? 'default' : 'assistant'}
								finishRendering={() => {}}
							>
								<Chat.MessageContent>{message.content}</Chat.MessageContent>
								{#if message.sources && message.sources.length > 0}
									<Chat.MessageFooter>
										<Chat.MessageSource>
											{#each message.sources as source}
												{@html source}
											{/each}
										</Chat.MessageSource>
									</Chat.MessageFooter>
								{/if}
							</Chat.Message>
						{/each}
					</Chat.Root>
				</div>
			</div>
		{/snippet}
	</StorybookContainer>
</Story>

<Story name="Long Thread with Scrolling">
	<StorybookContainer class="p-0">
		{#snippet children()}
			<div class="h-[600px] w-full">
				<div class="h-full p-6">
					<h2 class="text-lg font-semibold mb-4 text-white">
						Long Thread (15+ messages with scrolling)
					</h2>
					<Chat.Root class="h-[500px]">
						{#each longThreadMessages as message}
							<Chat.Message
								variant={message.role === 'user' ? 'default' : 'assistant'}
								finishRendering={() => {}}
							>
								<Chat.MessageContent>{message.content}</Chat.MessageContent>
								{#if message.sources && message.sources.length > 0}
									<Chat.MessageFooter>
										<Chat.MessageSource>
											{#each message.sources as source}
												{@html source}
											{/each}
										</Chat.MessageSource>
									</Chat.MessageFooter>
								{/if}
							</Chat.Message>
						{/each}
					</Chat.Root>
				</div>
			</div>
		{/snippet}
	</StorybookContainer>
</Story>

<Story name="Mixed Content with Sources">
	<StorybookContainer class="p-0">
		{#snippet children()}
			<div class="h-[600px] w-full">
				<div class="h-full p-6">
					<h2 class="text-lg font-semibold mb-4 text-white">
						Mixed Content with Various Sources
					</h2>
					<Chat.Root class="h-[500px]">
						{#each mixedContentMessages as message}
							<Chat.Message
								variant={message.role === 'user' ? 'default' : 'assistant'}
								finishRendering={() => {}}
							>
								<Chat.MessageContent>{message.content}</Chat.MessageContent>
								{#if message.sources && message.sources.length > 0}
									<Chat.MessageFooter>
										<Chat.MessageSource>
											{#each message.sources as source}
												{@html source}
											{/each}
										</Chat.MessageSource>
									</Chat.MessageFooter>
								{/if}
							</Chat.Message>
						{/each}
					</Chat.Root>
				</div>
			</div>
		{/snippet}
	</StorybookContainer>
</Story>

<Story name="Interactive Chat Behavior">
	<StorybookContainer class="p-0">
		{#snippet children()}
			<div class="h-[600px] w-full">
				<div class="h-full p-6">
					<h2 class="text-lg font-semibold mb-4 text-white">
						Interactive Chat with Auto-scroll
					</h2>
					<Chat.Root class="h-[500px]">
						{#each interactiveMessages as message}
							<Chat.Message
								variant={message.role === 'user' ? 'default' : 'assistant'}
								finishRendering={() => {}}
							>
								<Chat.MessageContent>{message.content}</Chat.MessageContent>
								{#if message.sources && message.sources.length > 0}
									<Chat.MessageFooter>
										<Chat.MessageSource>
											{#each message.sources as source}
												{@html source}
											{/each}
										</Chat.MessageSource>
									</Chat.MessageFooter>
								{/if}
							</Chat.Message>
						{/each}
					</Chat.Root>
				</div>
			</div>
		{/snippet}
	</StorybookContainer>
</Story>
