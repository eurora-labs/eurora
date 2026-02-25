<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { getAttachmentItemContext } from './attachments-context.svelte.js';
	import ImageIcon from '@lucide/svelte/icons/image';
	import VideoIcon from '@lucide/svelte/icons/video';
	import Music2Icon from '@lucide/svelte/icons/music-2';
	import FileTextIcon from '@lucide/svelte/icons/file-text';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import PaperclipIcon from '@lucide/svelte/icons/paperclip';
	import type { Snippet } from 'svelte';

	interface Props {
		class?: string;
		fallbackIcon?: Snippet;
	}

	let { class: className, fallbackIcon, ...restProps }: Props = $props();

	let ctx = getAttachmentItemContext();
	let data = $derived(ctx.data);
	let mediaCategory = $derived(ctx.mediaCategory);
	let variant = $derived(ctx.variant);

	let iconSize = $derived(variant === 'inline' ? 'size-3' : 'size-4');

	const mediaCategoryIcons = {
		image: ImageIcon,
		video: VideoIcon,
		audio: Music2Icon,
		document: FileTextIcon,
		source: GlobeIcon,
		unknown: PaperclipIcon,
	} as const;
</script>

<div
	data-slot="attachment-preview"
	class={cn(
		'flex shrink-0 items-center justify-center overflow-hidden',
		variant === 'grid' && 'size-full bg-muted',
		variant === 'inline' && 'size-5 rounded bg-background',
		variant === 'list' && 'size-12 rounded bg-muted',
		className,
	)}
	{...restProps}
>
	{#if mediaCategory === 'image' && data.type === 'file' && data.url}
		{#if variant === 'grid'}
			<img
				alt={data.filename || 'Image'}
				class="size-full object-cover"
				height={96}
				src={data.url}
				width={96}
			/>
		{:else}
			<img
				alt={data.filename || 'Image'}
				class="size-full rounded object-cover"
				height={20}
				src={data.url}
				width={20}
			/>
		{/if}
	{:else if mediaCategory === 'video' && data.type === 'file' && data.url}
		<video class="size-full object-cover" muted src={data.url}></video>
	{:else if fallbackIcon}
		{@render fallbackIcon()}
	{:else}
		{@const Icon = mediaCategoryIcons[mediaCategory]}
		<Icon class={cn(iconSize, 'text-muted-foreground')} />
	{/if}
</div>
