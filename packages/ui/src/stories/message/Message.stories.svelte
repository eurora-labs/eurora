<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import { Root as MessageRoot } from '$lib/components/ai-elements/message/index';

	const { Story } = defineMeta({
		title: 'AI Elements / Message',
		component: MessageRoot,
		parameters: {
			layout: 'padded',
			controls: { disable: true },
			docs: {
				description: {
					component:
						'Message component showcasing user and assistant messages with branching, attachments, toolbar actions, and markdown rendering.',
				},
			},
		},
	});
</script>

<script lang="ts">
	import * as Message from '$lib/components/ai-elements/message/index';
	import * as Attachment from '$lib/components/ai-elements/attachments/index';
	import { Provider as TooltipProvider } from '$lib/components/tooltip/index';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import RefreshCcwIcon from '@lucide/svelte/icons/refresh-ccw';
	import ThumbsUpIcon from '@lucide/svelte/icons/thumbs-up';
	import ThumbsDownIcon from '@lucide/svelte/icons/thumbs-down';

	let idCounter = 0;
	function uid() {
		return `id-${++idCounter}`;
	}

	interface MessageType {
		key: string;
		from: 'user' | 'assistant';
		versions: { id: string; content: string }[];
		attachments?: {
			id: string;
			type: 'file';
			url: string;
			mediaType?: string;
			filename?: string;
		}[];
	}

	const messages: MessageType[] = [
		{
			key: uid(),
			from: 'user',
			attachments: [
				{
					id: uid(),
					type: 'file',
					url: 'https://images.unsplash.com/photo-1506905925346-21bda4d32df4?w=400&h=400&fit=crop',
					mediaType: 'image/jpeg',
					filename: 'palace-of-fine-arts.jpg',
				},
				{
					id: uid(),
					type: 'file',
					url: '',
					mediaType: 'application/pdf',
					filename: 'react-hooks-guide.pdf',
				},
			],
			versions: [
				{
					id: uid(),
					content: 'How do React hooks work and when should I use them?',
				},
			],
		},
		{
			key: uid(),
			from: 'assistant',
			versions: [
				{
					id: uid(),
					content: `# React Hooks Guide

React hooks are functions that let you "hook into" React state and lifecycle features from function components. Here's what you need to know:

## Core Hooks

### useState
Adds state to functional components:

\`\`\`jsx
const [count, setCount] = useState(0);

return (
  <button onClick={() => setCount(count + 1)}>
    Count: {count}
  </button>
);
\`\`\`

### useEffect
Handles side effects (data fetching, subscriptions, DOM updates):

\`\`\`jsx
useEffect(() => {
  document.title = \`You clicked \${count} times\`;

  // Cleanup function (optional)
  return () => {
    document.title = 'React App';
  };
}, [count]); // Dependency array
\`\`\`

## When to Use Hooks

- ✅ **Function components** - Hooks only work in function components
- ✅ **Replacing class components** - Modern React favors hooks over classes
- ✅ **Sharing stateful logic** - Create custom hooks to reuse logic
- ❌ **Class components** - Use lifecycle methods instead

## Rules of Hooks

1. Only call hooks at the **top level** (not inside loops, conditions, or nested functions)
2. Only call hooks from **React functions** (components or custom hooks)

Would you like to explore more advanced hooks like \`useCallback\` or \`useMemo\`?`,
				},
				{
					id: uid(),
					content: `React hooks are special functions that let you use React features in function components. The most common ones are:

- **useState** - for managing component state
- **useEffect** - for side effects like data fetching
- **useContext** - for consuming context values
- **useRef** - for accessing DOM elements

Here's a simple example:

\`\`\`jsx
function Counter() {
  const [count, setCount] = useState(0);

  return (
    <button onClick={() => setCount(count + 1)}>
      Clicked {count} times
    </button>
  );
}
\`\`\`

Which specific hook would you like to learn more about?`,
				},
				{
					id: uid(),
					content: `Absolutely! React hooks are a powerful feature introduced in React 16.8. They solve several problems:

## Key Benefits

1. **Simpler code** - No need for class components
2. **Reusable logic** - Extract stateful logic into custom hooks
3. **Better organization** - Group related code together

## Most Popular Hooks

| Hook | Purpose |
|------|---------|
| useState | Add state to components |
| useEffect | Handle side effects |
| useContext | Access context values |
| useReducer | Complex state logic |
| useCallback | Memoize functions |
| useMemo | Memoize values |

The beauty of hooks is that they let you reuse stateful logic without changing your component hierarchy. Want to dive into a specific hook?`,
				},
			],
		},
	];

	let liked = $state<Record<string, boolean>>({});
	let disliked = $state<Record<string, boolean>>({});
	let activeBranch = $state<Record<string, number>>({});

	function handleCopy(content: string) {
		navigator.clipboard.writeText(content);
	}

	function handleRetry() {
		console.log('Retrying...');
	}

	function toggleLike(key: string) {
		liked = { ...liked, [key]: !liked[key] };
	}

	function toggleDislike(key: string) {
		disliked = { ...disliked, [key]: !disliked[key] };
	}
</script>

<Story name="Message">
	<TooltipProvider>
		<div class="flex w-175 flex-col gap-4">
			{#each messages as msg (msg.key)}
				{#if msg.versions.length > 1}
					<Message.Branch
						defaultBranch={0}
						onBranchChange={(i) => (activeBranch = { ...activeBranch, [msg.key]: i })}
					>
						<Message.BranchContent count={msg.versions.length}>
							{#snippet children({ index })}
								{@const version = msg.versions[index]}
								<Message.Root from={msg.from}>
									<Message.Content>
										<Message.Response content={version.content} />
									</Message.Content>
								</Message.Root>
							{/snippet}
						</Message.BranchContent>
						{#if msg.from === 'assistant'}
							<Message.Toolbar>
								<Message.BranchSelector>
									<Message.BranchPrevious />
									<Message.BranchPage />
									<Message.BranchNext />
								</Message.BranchSelector>
								<Message.Actions>
									<Message.Action
										label="Retry"
										onclick={handleRetry}
										tooltip="Regenerate response"
									>
										<RefreshCcwIcon class="size-4" />
									</Message.Action>
									<Message.Action
										label="Like"
										onclick={() => toggleLike(msg.key)}
										tooltip="Like this response"
									>
										<ThumbsUpIcon
											class="size-4"
											fill={liked[msg.key] ? 'currentColor' : 'none'}
										/>
									</Message.Action>
									<Message.Action
										label="Dislike"
										onclick={() => toggleDislike(msg.key)}
										tooltip="Dislike this response"
									>
										<ThumbsDownIcon
											class="size-4"
											fill={disliked[msg.key] ? 'currentColor' : 'none'}
										/>
									</Message.Action>
									<Message.Action
										label="Copy"
										onclick={() =>
											handleCopy(
												msg.versions[activeBranch[msg.key] ?? 0]?.content ||
													'',
											)}
										tooltip="Copy to clipboard"
									>
										<CopyIcon class="size-4" />
									</Message.Action>
								</Message.Actions>
							</Message.Toolbar>
						{/if}
					</Message.Branch>
				{:else}
					{@const version = msg.versions[0]}
					<Message.Root from={msg.from}>
						{#if msg.attachments?.length}
							<Attachment.Root class="mb-2" variant="grid">
								{#each msg.attachments as attachment (attachment.id)}
									<Attachment.Item data={attachment}>
										<Attachment.Preview />
										<Attachment.Remove />
									</Attachment.Item>
								{/each}
							</Attachment.Root>
						{/if}
						<Message.Content>
							{#if msg.from === 'assistant'}
								<Message.Response content={version.content} />
							{:else}
								{version.content}
							{/if}
						</Message.Content>
						{#if msg.from === 'assistant'}
							<Message.Actions>
								<Message.Action
									label="Retry"
									onclick={handleRetry}
									tooltip="Regenerate response"
								>
									<RefreshCcwIcon class="size-4" />
								</Message.Action>
								<Message.Action
									label="Like"
									onclick={() => toggleLike(msg.key)}
									tooltip="Like this response"
								>
									<ThumbsUpIcon
										class="size-4"
										fill={liked[msg.key] ? 'currentColor' : 'none'}
									/>
								</Message.Action>
								<Message.Action
									label="Dislike"
									onclick={() => toggleDislike(msg.key)}
									tooltip="Dislike this response"
								>
									<ThumbsDownIcon
										class="size-4"
										fill={disliked[msg.key] ? 'currentColor' : 'none'}
									/>
								</Message.Action>
								<Message.Action
									label="Copy"
									onclick={() => handleCopy(version.content)}
									tooltip="Copy to clipboard"
								>
									<CopyIcon class="size-4" />
								</Message.Action>
							</Message.Actions>
						{/if}
					</Message.Root>
				{/if}
			{/each}
		</div>
	</TooltipProvider>
</Story>
