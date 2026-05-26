<script lang="ts">
	import { ACTIVITY_SERVICE } from '$lib/services/activity-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Timeline from '@eurora/ui/custom-components/timeline/index';
	import ArrowUpToLineIcon from '@lucide/svelte/icons/arrow-up-to-line';
	import { onMount } from 'svelte';

	const activityService = inject(ACTIVITY_SERVICE);

	// Tag <body> while the rail is mounted so peer chrome (sidebar, titlebar
	// leading region) can shift over to make room. A body class is more
	// portable across webviews than a `:has()` selector and survives the
	// Svelte component's own scope.
	onMount(() => {
		document.body.classList.add('has-timeline-rail');
		return () => document.body.classList.remove('has-timeline-rail');
	});

	let scrollContainer: HTMLDivElement | undefined = $state();
	let sentinel: HTMLLIElement | undefined = $state();
	let scrollTop = $state(0);

	// The jump-to-live affordance overlays the top of the rail whenever
	// the user has either picked an older row (`activeIndex > 0`) or
	// scrolled the viewport down. Either signal alone is enough — both
	// mean "the live row isn't where the eye lands right now."
	const showJumpToLive = $derived(activityService.activeIndex > 0 || scrollTop > 0);

	function onScroll(event: Event): void {
		scrollTop = (event.currentTarget as HTMLDivElement).scrollTop;
	}

	function jumpToLive(): void {
		activityService.jumpToLive();
		scrollContainer?.scrollTo({ top: 0, behavior: 'smooth' });
	}

	function scrollActiveIntoView(): void {
		const app = activityService.activeApp;
		if (!app) return;
		// `aria-activedescendant` already addresses the active row by
		// this id, so reusing it for the scroll target keeps the two in
		// lockstep — no parallel ref-tracking needed.
		const el = document.getElementById(`timeline-rail-item-${app.id}`);
		el?.scrollIntoView({ block: 'nearest' });
	}

	function onKeydown(event: KeyboardEvent): void {
		switch (event.key) {
			case 'ArrowDown':
			case 'PageDown':
				event.preventDefault();
				if (activityService.selectNext()) scrollActiveIntoView();
				break;
			case 'ArrowUp':
			case 'PageUp':
				event.preventDefault();
				if (activityService.selectPrevious()) scrollActiveIntoView();
				break;
			case 'Home':
				event.preventDefault();
				jumpToLive();
				break;
			case 'End':
				event.preventDefault();
				activityService.selectIndex(activityService.recent.length - 1);
				scrollActiveIntoView();
				break;
			default:
				break;
		}
	}

	// Bottom-sentinel observer triggers pagination as soon as the
	// invisible sentinel row gets within ~200px of the viewport. This
	// replaces the previous "active index near the loaded edge" prefetch
	// — with native scrolling, the right signal is viewport proximity,
	// not selection position. The observer re-attaches when either
	// element ref first resolves and tears down on unmount.
	$effect(() => {
		if (!scrollContainer || !sentinel) return;
		const observer = new IntersectionObserver(
			(entries) => {
				for (const entry of entries) {
					if (entry.isIntersecting) {
						void activityService.loadMore();
					}
				}
			},
			{ root: scrollContainer, rootMargin: '200px 0px 0px 0px' },
		);
		observer.observe(sentinel);
		return () => observer.disconnect();
	});
</script>

<aside
	data-slot="timeline-rail"
	class="bg-sidebar relative z-30 flex h-full w-12 shrink-0 flex-col overflow-hidden"
	aria-label="Recent activity"
>
	<div
		bind:this={scrollContainer}
		role="listbox"
		tabindex="0"
		aria-label="Recent activity"
		aria-activedescendant={activityService.activeApp
			? `timeline-rail-item-${activityService.activeApp.id}`
			: undefined}
		class="timeline-rail-scroll flex-1 overflow-x-hidden overflow-y-auto overscroll-contain focus:outline-none"
		onscroll={onScroll}
		onkeydown={onKeydown}
	>
		<Timeline.Root class="py-2">
			{#each activityService.recent as item, i (item.id)}
				<Timeline.Item
					id={`timeline-rail-item-${item.id}`}
					color={item.accent?.hex}
					active={i === activityService.activeIndex}
					iconSrc={item.iconBase64}
					name={item.name}
					onSelect={() => activityService.selectIndex(i)}
				/>
			{/each}
			<li bind:this={sentinel} aria-hidden="true" class="h-px w-full shrink-0"></li>
		</Timeline.Root>
	</div>

	{#if showJumpToLive}
		<Button
			variant="secondary"
			size="icon-sm"
			class="ring-border absolute top-2 left-1/2 z-10 -translate-x-1/2 rounded-full shadow-lg ring-1"
			aria-label="Jump to most-recent activity"
			onclick={jumpToLive}
		>
			<ArrowUpToLineIcon class="size-4" />
		</Button>
	{/if}
</aside>

<style>
	/* Hide the rail's vertical scrollbar. Scrolling still works via
	   wheel, trackpad, and keyboard — only the visual scrollbar track
	   is suppressed. Firefox honours `scrollbar-width`; Webkit/Blink
	   need the pseudo-element rule. */
	.timeline-rail-scroll {
		scrollbar-width: none;
	}
	.timeline-rail-scroll::-webkit-scrollbar {
		display: none;
	}
</style>
