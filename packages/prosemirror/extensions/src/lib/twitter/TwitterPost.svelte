<script lang="ts" module>
	import { Node as PMNode } from 'prosemirror-model';
	import type { NodeSpec } from 'prosemirror-model';

	export interface Tweet {
		text: string;
		timestamp?: string;
		author?: string;
	}

	export interface TweetAttrs {
		id?: string;
		url?: string;
		title?: string;
		tweets?: string;
		timestamp?: string;
		author?: string;
		text?: string;
	}

	export const tweetAttrs: TweetAttrs = {
		id: undefined,
		url: undefined,
		title: undefined,
		tweets: undefined,
		timestamp: undefined,
		author: undefined,
		text: undefined,
	};

	export const tweetSchema: NodeSpec = {
		attrs: Object.entries(tweetAttrs).reduce(
			(acc, [key, value]) => ({ ...acc, [key]: { default: value } }),
			{},
		),
		group: 'inline',
		inline: true,
		atom: true,
		selectable: true,

		parseDOM: [
			{
				tag: 'span.twitter-post',
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return {
							id: dom.getAttribute('id'),
							url: dom.getAttribute('data-url'),
							title: dom.getAttribute('data-title'),
							tweets: dom.getAttribute('data-tweets'),
							timestamp: dom.getAttribute('data-timestamp'),
							author: dom.getAttribute('data-author'),
							text: dom.getAttribute('data-text'),
						};
					}
					return null;
				},
			},
		],
		toDOM(node: PMNode) {
			const { id, url, title, tweets, timestamp, author, text } = node.attrs;
			return [
				'span',
				{
					id,
					'data-url': url,
					'data-title': title,
					'data-tweets': tweets,
					'data-timestamp': timestamp,
					'data-author': author,
					'data-text': text,
					class: 'twitter-post',
				},
			];
		},
	};
</script>

<script lang="ts">
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Popover from '@eurora/ui/components/popover/index';
	import { ContextChip } from '@eurora/ui/custom-components/context-chip/index';
	import { SiX } from '@icons-pack/svelte-simple-icons';
	import type { SvelteNodeViewProps } from '@eurora/prosemirror-core/index';

	export interface Props extends SvelteNodeViewProps<TweetAttrs> {
		ref: HTMLElement;
		attrs: TweetAttrs;
	}

	let { ref, attrs }: Props = $props();

	export { ref, attrs };

	let parsedTweets: Tweet[] = $state([]);

	$effect(() => {
		if (attrs.tweets) {
			try {
				parsedTweets = JSON.parse(attrs.tweets) as Tweet[];
			} catch {
				parsedTweets = [];
			}
		}
	});

	function handleClick(event: MouseEvent) {
		if (attrs.tweets) {
			try {
				const tweets = JSON.parse(attrs.tweets) as Tweet[];
				const tweetTexts = tweets.map((tweet) => tweet.text).join('\n\n');
				alert(`Twitter Content:\n\n${tweetTexts}`);
			} catch (_e) {
				alert('Twitter content available');
			}
		}
		event.preventDefault();
	}

	function handleKeyDown(event: KeyboardEvent) {
		event.preventDefault();
	}

	export function destroy() {
		ref?.remove();
	}

	function getDisplayText(): string {
		if (attrs.text) return attrs.text;
		if (attrs.tweets) {
			try {
				const tweets = JSON.parse(attrs.tweets) as Tweet[];
				if (tweets.length > 0) {
					return tweets[0].text.length > 50
						? tweets[0].text.substring(0, 50) + '...'
						: tweets[0].text;
				}
			} catch (_e) {}
		}
		return 'content';
	}

	function getTweetCount(): number {
		if (attrs.tweets) {
			try {
				const tweets = JSON.parse(attrs.tweets) as Tweet[];
				return tweets.length;
			} catch (_e) {
				return 0;
			}
		}
		return 0;
	}
</script>

<Popover.Root>
	<Popover.Trigger>
		<ContextChip bind:ref data-hole {...attrs} onkeydown={handleKeyDown} onclick={handleClick}>
			<SiX size={20} />
			{getDisplayText()}
			{#if getTweetCount() > 1}
				<span class="text-xs opacity-70">({getTweetCount()} tweets)</span>
			{/if}
		</ContextChip>
	</Popover.Trigger>
	<Popover.Content class="w-96">
		<div class="grid gap-4">
			<div class="space-y-2">
				<h4 class="font-medium leading-none">Twitter</h4>
				<p class="text-muted-foreground text-sm">View and manage Twitter content.</p>
			</div>
			<div class="grid gap-2">
				{#if attrs.url}
					<div class="grid grid-cols-3 items-center gap-4">
						<Label for="url">URL</Label>
						<Input id="url" value={attrs.url} class="col-span-2 h-8" readonly />
					</div>
				{/if}
				{#if attrs.title}
					<div class="grid grid-cols-3 items-center gap-4">
						<Label for="title">Title</Label>
						<Input id="title" value={attrs.title} class="col-span-2 h-8" readonly />
					</div>
				{/if}
				{#if attrs.author}
					<div class="grid grid-cols-3 items-center gap-4">
						<Label for="author">Author</Label>
						<Input id="author" value={attrs.author} class="col-span-2 h-8" readonly />
					</div>
				{/if}
				{#if attrs.timestamp}
					<div class="grid grid-cols-3 items-center gap-4">
						<Label for="timestamp">Timestamp</Label>
						<Input
							id="timestamp"
							value={attrs.timestamp}
							class="col-span-2 h-8"
							readonly
						/>
					</div>
				{/if}
				<div class="grid grid-cols-3 items-center gap-4">
					<Label for="tweetCount">Tweets</Label>
					<Input
						id="tweetCount"
						value={getTweetCount().toString()}
						class="col-span-2 h-8"
						readonly
					/>
				</div>
			</div>
			{#if attrs.tweets}
				<div class="space-y-2">
					<Label>Tweet Content</Label>
					<div class="max-h-48 overflow-y-auto border rounded p-2 text-sm">
						{#if parsedTweets && parsedTweets.length > 0}
							{#each parsedTweets as tweet, index}
								<div
									class="mb-2 pb-2 {index < parsedTweets.length - 1
										? 'border-b'
										: ''}"
								>
									<p class="text-sm">{tweet.text}</p>
									{#if tweet.author}
										<p class="text-xs text-muted-foreground mt-1">
											— {tweet.author}
										</p>
									{/if}
									{#if tweet.timestamp}
										<p class="text-xs text-muted-foreground">
											{tweet.timestamp}
										</p>
									{/if}
								</div>
							{/each}
						{:else}
							<p class="text-muted-foreground">Unable to parse tweet content</p>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</Popover.Content>
</Popover.Root>
