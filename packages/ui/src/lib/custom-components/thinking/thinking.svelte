<script lang="ts">
	import { cn } from '$lib/utils.js';

	export interface ThinkingProps {
		label?: string;
		class?: string;
	}

	let { label = 'Thinking', class: className }: ThinkingProps = $props();
</script>

<div
	role="status"
	aria-label={label}
	class={cn('thinking-root inline-flex items-center', className)}
>
	<span class="thinking-label" aria-hidden="true">
		{#each label.split('') as char, i}
			<span class="thinking-char" style="--i: {i}">{char}</span>
		{/each}
	</span>

	<span class="sr-only">{label}</span>
</div>

<style>
	.thinking-root {
		--duration: 1.6s;
		--stagger: 0.08s;
	}

	.thinking-char {
		display: inline-block;
		animation: ripple var(--duration) ease-in-out calc(var(--i) * var(--stagger)) infinite;
		opacity: 1;
	}

	@keyframes ripple {
		0% {
			opacity: 1;
		}
		15% {
			opacity: 0.5;
		}
		35% {
			opacity: 1;
		}
		100% {
			opacity: 1;
		}
	}
</style>
