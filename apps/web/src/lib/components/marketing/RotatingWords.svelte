<script lang="ts" module>
	export interface RotatingWordsProps {
		/** Words to cycle through. Providing one or zero disables rotation. */
		words: string[];
		/** Milliseconds each word stays visible before rolling to the next. */
		delay?: number;
		/** Roll animation duration in milliseconds. */
		duration?: number;
		/** When false, the rotation stops on the last word instead of cycling back. */
		loop?: boolean;
		/** Optional class applied to the wrapping element. */
		class?: string;
	}
</script>

<script lang="ts">
	import { cubicOut } from 'svelte/easing';
	import { MediaQuery } from 'svelte/reactivity';
	import { fly } from 'svelte/transition';

	let {
		words,
		delay = 2500,
		duration = 450,
		loop = true,
		class: className = '',
	}: RotatingWordsProps = $props();

	let index = $state(0);
	const reducedMotion = new MediaQuery('(prefers-reduced-motion: reduce)');

	$effect(() => {
		if (words.length <= 1) return;

		index = index % words.length;

		const id = setInterval(() => {
			const next = index + 1;
			if (next >= words.length && !loop) {
				clearInterval(id);
				return;
			}
			index = next % words.length;
		}, delay);

		return () => clearInterval(id);
	});

	function roll(node: Element, { y }: { y: string }) {
		return fly(node, {
			y: reducedMotion.current ? 0 : y,
			duration: reducedMotion.current ? 0 : duration,
			easing: cubicOut,
		});
	}
</script>

<span class="rotating-words {className}">
	<span class="sr-only">{words.join(' or ')}</span>

	<span class="rotating-words-sizer" aria-hidden="true">
		{#each words as word (word)}
			<span class="rotating-words-slot">{word}</span>
		{/each}
	</span>

	{#key index}
		<span
			class="rotating-words-slot rotating-words-active"
			aria-hidden="true"
			in:roll={{ y: '100%' }}
			out:roll={{ y: '-100%' }}
		>
			{words[index]}
		</span>
	{/key}
</span>

<style>
	.rotating-words {
		display: inline-grid;
		overflow: hidden;
		color: var(--color-orange-500);
		line-height: inherit;
		vertical-align: bottom;
	}

	.rotating-words-sizer {
		display: grid;
		visibility: hidden;
		grid-area: 1 / 1;
		pointer-events: none;
	}

	.rotating-words-slot {
		grid-area: 1 / 1;
	}
</style>
