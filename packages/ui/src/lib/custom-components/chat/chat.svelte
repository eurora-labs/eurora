<script lang="ts" module>
	export interface ChatProps {
		class?: string;
		children?: any;
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { onMount, tick } from 'svelte';
	let { class: className, children }: ChatProps = $props();

	let viewportRef = $state<HTMLDivElement>();
	let userScrolledUp = $state(false);
	const BOTTOM_THRESHOLD = 70;

	function isNearBottom(el: HTMLElement): boolean {
		return el.scrollHeight - el.scrollTop - el.clientHeight <= BOTTOM_THRESHOLD;
	}

	onMount(() => {
		if (!viewportRef) return;
		const el = viewportRef;

		const onWheel = (e: WheelEvent) => {
			if (e.deltaY < 0) {
				userScrolledUp = true;
			}
		};

		let touchStartY = 0;
		const onTouchStart = (e: TouchEvent) => {
			touchStartY = e.touches[0].clientY;
		};
		const onTouchMove = (e: TouchEvent) => {
			if (e.touches[0].clientY > touchStartY) {
				userScrolledUp = true;
			}
		};

		const onScroll = () => {
			if (userScrolledUp && isNearBottom(el)) {
				userScrolledUp = false;
			}
		};

		el.addEventListener('wheel', onWheel, { passive: true });
		el.addEventListener('touchstart', onTouchStart, { passive: true });
		el.addEventListener('touchmove', onTouchMove, { passive: true });
		el.addEventListener('scroll', onScroll, { passive: true });

		return () => {
			el.removeEventListener('wheel', onWheel);
			el.removeEventListener('touchstart', onTouchStart);
			el.removeEventListener('touchmove', onTouchMove);
			el.removeEventListener('scroll', onScroll);
		};
	});

	export async function scrollToBottom() {
		if (userScrolledUp) return;
		await tick();
		if (!viewportRef) return;
		viewportRef.scrollTop = viewportRef.scrollHeight;
	}
</script>

<div bind:this={viewportRef} class={cn('overflow-y-auto', className)}>
	<div class="space-y-4 p-4 pb-0 flex flex-col">
		{@render children?.()}
	</div>
</div>
