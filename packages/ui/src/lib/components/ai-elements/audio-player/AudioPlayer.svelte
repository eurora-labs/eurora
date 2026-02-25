<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import 'media-chrome';

	interface Props extends HTMLAttributes<HTMLElement> {
		children?: Snippet;
	}

	let { children, style, ...restProps }: Props = $props();

	const mediaStyles = [
		'--media-background-color: transparent',
		'--media-button-icon-height: 1rem',
		'--media-button-icon-width: 1rem',
		'--media-control-background: transparent',
		'--media-control-hover-background: var(--color-accent)',
		'--media-control-padding: 0',
		'--media-font: var(--font-sans)',
		'--media-font-size: 10px',
		'--media-icon-color: currentColor',
		'--media-preview-time-background: var(--color-background)',
		'--media-preview-time-border-radius: var(--radius-md)',
		'--media-preview-time-text-shadow: none',
		'--media-primary-color: var(--color-primary)',
		'--media-range-bar-color: var(--color-primary)',
		'--media-range-track-background: var(--color-secondary)',
		'--media-secondary-color: var(--color-secondary)',
		'--media-text-color: var(--color-foreground)',
		'--media-tooltip-arrow-display: none',
		'--media-tooltip-background: var(--color-background)',
		'--media-tooltip-border-radius: var(--radius-md)',
	].join('; ');

	const mergedStyle = $derived(style ? `${mediaStyles}; ${style}` : mediaStyles);
</script>

<media-controller audio data-slot="audio-player" style={mergedStyle} {...restProps}>
	{@render children?.()}
</media-controller>
