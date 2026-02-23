<script lang="ts">
	// TODO: Integrate @rive-app/canvas for Rive animations
	import type { HTMLCanvasAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';
	import { onMount } from 'svelte';
	import {
		PersonaContext,
		setPersonaContext,
		type PersonaVariant,
		type PersonaState,
	} from './persona-context.svelte.js';

	interface Props extends HTMLCanvasAttributes {
		variant?: PersonaVariant;
		state?: PersonaState;
		onLoad?: () => void;
		onLoadError?: (error: unknown) => void;
		onReady?: () => void;
	}

	let {
		class: className,
		variant = 'obsidian',
		state = 'idle',
		onLoad,
		onLoadError,
		onReady,
		...restProps
	}: Props = $props();

	let context = new PersonaContext({ variant, state });
	setPersonaContext(context);

	$effect(() => {
		context.variant = variant;
	});

	$effect(() => {
		context.state = state;
	});

	let canvasRef: HTMLCanvasElement | undefined = $state();

	$effect(() => {
		if (!context.source.dynamicColor) return;

		const isDark =
			document.documentElement.classList.contains('dark') ||
			window.matchMedia?.('(prefers-color-scheme: dark)').matches;

		context.colorRgb = isDark ? [255, 255, 255] : [0, 0, 0];

		const observer = new MutationObserver(() => {
			const dark = document.documentElement.classList.contains('dark');
			context.colorRgb = dark ? [255, 255, 255] : [0, 0, 0];
		});

		observer.observe(document.documentElement, {
			attributeFilter: ['class'],
			attributes: true,
		});

		const mql = window.matchMedia?.('(prefers-color-scheme: dark)');
		const handleMediaChange = () => {
			const dark =
				document.documentElement.classList.contains('dark') || mql?.matches;
			context.colorRgb = dark ? [255, 255, 255] : [0, 0, 0];
		};
		mql?.addEventListener('change', handleMediaChange);

		return () => {
			observer.disconnect();
			mql?.removeEventListener('change', handleMediaChange);
		};
	});

	onMount(() => {
		// TODO: Initialize Rive canvas with context.source.source
		// and configure state machine inputs for listening/thinking/speaking/asleep
		onReady?.();
	});
</script>

<canvas
	data-slot="persona"
	bind:this={canvasRef}
	class={cn('size-16 shrink-0', className)}
	{...restProps}
></canvas>
