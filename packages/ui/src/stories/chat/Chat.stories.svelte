<script module lang="ts">
	import { defineMeta } from '@storybook/addon-svelte-csf';
	import {
		Conversation,
		ConversationContent,
		ConversationScrollButton,
	} from '$lib/components/ai-elements/conversation/index';

	const { Story } = defineMeta({
		title: 'AI Elements / Chat',
		component: Conversation,
		parameters: {
			layout: 'fullscreen',
			controls: { disable: true },
			docs: {
				description: {
					component:
						'Full chat interface demo showcasing conversation, message branching, sources, reasoning, suggestions, attachments, model selector, and prompt input.',
				},
			},
		},
	});
</script>

<script lang="ts">
	import {
		ConversationContent as ConvContent,
		ConversationScrollButton as ConvScrollButton,
	} from '$lib/components/ai-elements/conversation/index';
	import {
		Message,
		MessageBranch,
		MessageBranchContent,
		MessageBranchSelector,
		MessageBranchPrevious,
		MessageBranchPage,
		MessageBranchNext,
		MessageContent,
		MessageResponse,
	} from '$lib/components/ai-elements/message/index';
	import {
		PromptInput,
		PromptInputBody,
		PromptInputTextarea,
		PromptInputHeader,
		PromptInputFooter,
		PromptInputTools,
		PromptInputButton,
		PromptInputSubmit,
		PromptInputActionMenu,
		PromptInputActionMenuTrigger,
		PromptInputActionMenuContent,
		PromptInputActionAddAttachments,
		usePromptInputAttachments,
		type PromptInputMessage,
		type ChatStatus,
	} from '$lib/components/ai-elements/prompt-input/index';
	import {
		Attachments,
		Attachment,
		AttachmentPreview,
		AttachmentRemove,
	} from '$lib/components/ai-elements/attachments/index';
	import { Shimmer } from '$lib/components/ai-elements/shimmer/index';
	import { Suggestions, Suggestion } from '$lib/components/ai-elements/suggestion/index';
	import {
		Sources,
		SourcesTrigger,
		SourcesContent,
		Source,
	} from '$lib/components/ai-elements/sources/index';
	import {
		Reasoning,
		ReasoningTrigger,
		ReasoningContent,
	} from '$lib/components/ai-elements/reasoning/index';
	import {
		ModelSelector,
		ModelSelectorTrigger,
		ModelSelectorContent,
		ModelSelectorInput,
		ModelSelectorList,
		ModelSelectorEmpty,
		ModelSelectorGroup,
		ModelSelectorItem,
		ModelSelectorLogo,
		ModelSelectorLogoGroup,
		ModelSelectorName,
	} from '$lib/components/ai-elements/model-selector/index';
	import { SpeechInput } from '$lib/components/ai-elements/speech-input/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	let idCounter = 0;
	function uid() {
		return `id-${++idCounter}`;
	}

	interface MessageType {
		key: string;
		from: 'user' | 'assistant';
		sources?: { href: string; title: string }[];
		versions: { id: string; content: string }[];
		reasoning?: { content: string; duration: number };
	}

	const initialMessages: MessageType[] = [
		{
			from: 'user',
			key: uid(),
			versions: [
				{ content: 'Can you explain how to use React hooks effectively?', id: uid() },
			],
		},
		{
			from: 'assistant',
			key: uid(),
			sources: [
				{ href: 'https://react.dev/reference/react', title: 'React Documentation' },
				{ href: 'https://react.dev/reference/react-dom', title: 'React DOM Documentation' },
			],
			versions: [
				{
					content: `# React Hooks Best Practices

React hooks are a powerful feature that let you use state and other React features without writing classes. Here are some tips for using them effectively:

## Rules of Hooks

1. **Only call hooks at the top level** of your component or custom hooks
2. **Don't call hooks inside loops, conditions, or nested functions**

## Common Hooks

- **useState**: For local component state
- **useEffect**: For side effects like data fetching
- **useContext**: For consuming context
- **useReducer**: For complex state logic
- **useCallback**: For memoizing functions
- **useMemo**: For memoizing values

## Example of useState and useEffect

\`\`\`jsx
function ProfilePage({ userId }) {
  const [user, setUser] = useState(null);

  useEffect(() => {
    fetchUser(userId).then(userData => {
      setUser(userData);
    });
  }, [userId]);

  return user ? <Profile user={user} /> : <Loading />;
}
\`\`\`

Would you like me to explain any specific hook in more detail?`,
					id: uid(),
				},
			],
		},
		{
			from: 'user',
			key: uid(),
			versions: [
				{
					content:
						'Yes, could you explain useCallback and useMemo in more detail? When should I use one over the other?',
					id: uid(),
				},
				{
					content:
						"I'm particularly interested in understanding the performance implications of useCallback and useMemo. Could you break down when each is most appropriate?",
					id: uid(),
				},
				{
					content:
						'Thanks for the overview! Could you dive deeper into the specific use cases where useCallback and useMemo make the biggest difference in React applications?',
					id: uid(),
				},
			],
		},
		{
			from: 'assistant',
			key: uid(),
			reasoning: {
				content: `The user is asking for a detailed explanation of useCallback and useMemo. I should provide a clear and concise explanation of each hook's purpose and how they differ.

The useCallback hook is used to memoize functions to prevent unnecessary re-renders of child components that receive functions as props.

The useMemo hook is used to memoize values to avoid expensive recalculations on every render.

Both hooks help with performance optimization, but they serve different purposes.`,
				duration: 10,
			},
			versions: [
				{
					content: `## useCallback vs useMemo

Both hooks help with performance optimization, but they serve different purposes:

### useCallback

\`useCallback\` memoizes **functions** to prevent unnecessary re-renders of child components that receive functions as props.

\`\`\`jsx
// Without useCallback - a new function is created on every render
const handleClick = () => {
  console.log(count);
};

// With useCallback - the function is only recreated when dependencies change
const handleClick = useCallback(() => {
  console.log(count);
}, [count]);
\`\`\`

### useMemo

\`useMemo\` memoizes **values** to avoid expensive recalculations on every render.

\`\`\`jsx
// Without useMemo - expensive calculation runs on every render
const sortedList = expensiveSort(items);

// With useMemo - calculation only runs when items change
const sortedList = useMemo(() => expensiveSort(items), [items]);
\`\`\`

### When to use which?

- Use **useCallback** when:
  - Passing callbacks to optimized child components that rely on reference equality
  - Working with event handlers that you pass to child components

- Use **useMemo** when:
  - You have computationally expensive calculations
  - You want to avoid recreating objects that are used as dependencies for other hooks

### Performance Note

Don't overuse these hooks! They come with their own overhead. Only use them when you have identified a genuine performance issue.`,
					id: uid(),
				},
			],
		},
	];

	const models = [
		{
			chef: 'OpenAI',
			chefSlug: 'openai',
			id: 'gpt-4o',
			name: 'GPT-4o',
			providers: ['openai', 'azure'],
		},
		{
			chef: 'OpenAI',
			chefSlug: 'openai',
			id: 'gpt-4o-mini',
			name: 'GPT-4o Mini',
			providers: ['openai', 'azure'],
		},
		{
			chef: 'Anthropic',
			chefSlug: 'anthropic',
			id: 'claude-opus-4-20250514',
			name: 'Claude 4 Opus',
			providers: ['anthropic', 'azure', 'google', 'amazon-bedrock'],
		},
		{
			chef: 'Anthropic',
			chefSlug: 'anthropic',
			id: 'claude-sonnet-4-20250514',
			name: 'Claude 4 Sonnet',
			providers: ['anthropic', 'azure', 'google', 'amazon-bedrock'],
		},
		{
			chef: 'Google',
			chefSlug: 'google',
			id: 'gemini-2.0-flash-exp',
			name: 'Gemini 2.0 Flash',
			providers: ['google'],
		},
	];

	const chefs = ['OpenAI', 'Anthropic', 'Google'];

	const suggestionsList = [
		'What are the latest trends in AI?',
		'How does machine learning work?',
		'Explain quantum computing',
		'Best practices for React development',
		'Tell me about TypeScript benefits',
		'How to optimize database queries?',
		'What is the difference between SQL and NoSQL?',
		'Explain cloud computing basics',
	];

	const mockResponses = [
		"That's a great question! Let me help you understand this concept better. The key thing to remember is that proper implementation requires careful consideration of the underlying principles and best practices in the field.",
		"I'd be happy to explain this topic in detail. From my understanding, there are several important factors to consider when approaching this problem. Let me break it down step by step for you.",
		"This is an interesting topic that comes up frequently. The solution typically involves understanding the core concepts and applying them in the right context. Here's what I recommend...",
		"Great choice of topic! This is something that many developers encounter. The approach I'd suggest is to start with the fundamentals and then build up to more complex scenarios.",
		"That's definitely worth exploring. From what I can see, the best way to handle this is to consider both the theoretical aspects and practical implementation details.",
	];

	const delay = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

	let model = $state(models[0].id);
	let modelSelectorOpen = $state(false);
	let useWebSearch = $state(false);
	let status = $state<ChatStatus>('ready');
	let messages = $state<MessageType[]>(initialMessages);

	let selectedModelData = $derived(models.find((m) => m.id === model));

	function updateMessageContent(messageId: string, newContent: string) {
		messages = messages.map((msg) => {
			if (msg.versions.some((v) => v.id === messageId)) {
				return {
					...msg,
					versions: msg.versions.map((v) =>
						v.id === messageId ? { ...v, content: newContent } : v,
					),
				};
			}
			return msg;
		});
	}

	async function streamResponse(messageId: string, content: string) {
		status = 'streaming';
		const words = content.split(' ');
		let currentContent = '';

		for (const [i, word] of words.entries()) {
			currentContent += (i > 0 ? ' ' : '') + word;
			updateMessageContent(messageId, currentContent);
			await delay(Math.random() * 100 + 50);
		}

		status = 'ready';
	}

	function addUserMessage(content: string) {
		const userMessage: MessageType = {
			from: 'user',
			key: `user-${Date.now()}`,
			versions: [{ content, id: `user-${Date.now()}` }],
		};

		messages = [...messages, userMessage];

		setTimeout(() => {
			const assistantMessageId = `assistant-${Date.now()}`;
			const randomResponse = mockResponses[Math.floor(Math.random() * mockResponses.length)];

			const assistantMessage: MessageType = {
				from: 'assistant',
				key: `assistant-${Date.now()}`,
				versions: [{ content: '', id: assistantMessageId }],
			};

			messages = [...messages, assistantMessage];
			streamResponse(assistantMessageId, randomResponse);
		}, 500);
	}

	function handleSubmit(message: PromptInputMessage) {
		const hasText = Boolean(message.text);
		const hasAttachments = Boolean(message.files?.length);
		if (!(hasText || hasAttachments)) return;

		status = 'submitted';
		addUserMessage(message.text || 'Sent with attachments');
	}

	function handleSuggestionClick(suggestion: string) {
		status = 'submitted';
		addUserMessage(suggestion);
	}

	function handleModelSelect(modelId: string) {
		model = modelId;
		modelSelectorOpen = false;
	}
</script>

<Story name="Chat">
	<div
		class="relative flex h-[700px] w-[900px] flex-col divide-y overflow-hidden rounded-lg border"
	>
		<Conversation class="min-h-0 flex-1">
			<ConvContent>
				{#each messages as msg (msg.key)}
					{#if msg.versions.length > 1}
						<MessageBranch defaultBranch={0}>
							<MessageBranchContent count={msg.versions.length}>
								{#snippet children({ index })}
									{@const version = msg.versions[index]}
									<Message from={msg.from}>
										<MessageContent>
											{#if version.content.trim().length > 0}
												<MessageResponse content={version.content} />
											{:else}
												<Shimmer>Thinking</Shimmer>
											{/if}
										</MessageContent>
									</Message>
								{/snippet}
							</MessageBranchContent>
							<MessageBranchSelector>
								<MessageBranchPrevious />
								<MessageBranchPage />
								<MessageBranchNext />
							</MessageBranchSelector>
						</MessageBranch>
					{:else}
						{@const version = msg.versions[0]}
						<Message from={msg.from}>
							{#if msg.sources?.length}
								<Sources>
									<SourcesTrigger count={msg.sources.length} />
									<SourcesContent>
										{#each msg.sources as source (source.href)}
											<Source href={source.href} title={source.title} />
										{/each}
									</SourcesContent>
								</Sources>
							{/if}
							{#if msg.reasoning}
								<Reasoning duration={msg.reasoning.duration}>
									<ReasoningTrigger />
									<ReasoningContent children={msg.reasoning.content} />
								</Reasoning>
							{/if}
							<MessageContent>
								{#if version.content.trim().length > 0}
									<MessageResponse content={version.content} />
								{:else}
									<Shimmer>Thinking</Shimmer>
								{/if}
							</MessageContent>
						</Message>
					{/if}
				{/each}
			</ConvContent>
			<ConvScrollButton />
		</Conversation>
		<div class="grid shrink-0 gap-4 pt-4">
			<Suggestions class="px-4">
				{#each suggestionsList as suggestion}
					<Suggestion {suggestion} onclick={handleSuggestionClick} />
				{/each}
			</Suggestions>
			<div class="w-full px-4 pb-4">
				<PromptInput globalDrop multiple onSubmit={handleSubmit}>
					<PromptInputHeader />
					<PromptInputBody>
						<PromptInputTextarea />
					</PromptInputBody>
					<PromptInputFooter>
						<PromptInputTools>
							<PromptInputActionMenu>
								<PromptInputActionMenuTrigger />
								<PromptInputActionMenuContent>
									<PromptInputActionAddAttachments />
								</PromptInputActionMenuContent>
							</PromptInputActionMenu>
							<SpeechInput class="shrink-0" />
							<PromptInputButton
								size="sm"
								onclick={() => (useWebSearch = !useWebSearch)}
								variant={useWebSearch ? 'default' : 'ghost'}
							>
								<GlobeIcon size={16} />
								<span>Search</span>
							</PromptInputButton>
							<ModelSelector bind:open={modelSelectorOpen}>
								<ModelSelectorTrigger>
									<PromptInputButton size="sm">
										{#if selectedModelData?.chefSlug}
											<ModelSelectorLogo
												provider={selectedModelData.chefSlug}
											/>
										{/if}
										{#if selectedModelData?.name}
											<ModelSelectorName
												>{selectedModelData.name}</ModelSelectorName
											>
										{/if}
									</PromptInputButton>
								</ModelSelectorTrigger>
								<ModelSelectorContent>
									<ModelSelectorInput placeholder="Search models..." />
									<ModelSelectorList>
										<ModelSelectorEmpty>No models found.</ModelSelectorEmpty>
										{#each chefs as chef}
											<ModelSelectorGroup heading={chef}>
												{#each models.filter((m) => m.chef === chef) as m (m.id)}
													<ModelSelectorItem
														value={m.id}
														onSelect={() => handleModelSelect(m.id)}
													>
														<ModelSelectorLogo provider={m.chefSlug} />
														<ModelSelectorName
															>{m.name}</ModelSelectorName
														>
														<ModelSelectorLogoGroup>
															{#each m.providers as provider}
																<ModelSelectorLogo {provider} />
															{/each}
														</ModelSelectorLogoGroup>
														{#if model === m.id}
															<CheckIcon class="ml-auto size-4" />
														{:else}
															<div class="ml-auto size-4"></div>
														{/if}
													</ModelSelectorItem>
												{/each}
											</ModelSelectorGroup>
										{/each}
									</ModelSelectorList>
								</ModelSelectorContent>
							</ModelSelector>
						</PromptInputTools>
						<PromptInputSubmit {status} />
					</PromptInputFooter>
				</PromptInput>
			</div>
		</div>
	</div>
</Story>
