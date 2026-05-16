<script lang="ts">
	import { ACTIVITY_SERVICE } from '$lib/services/activity-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import * as ScrollArea from '@eurora/ui/components/scroll-area/index';
	import * as Timeline from '@eurora/ui/custom-components/timeline/index';
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

	// The Rust-side accent classifier emits `iconBg` as a binary signal
	// ('#000000' = icon glyph is mostly-white and wants a dark backdrop;
	// '#ffffff' = mostly-black, wants a light backdrop). Map that signal
	// to the theme-agnostic surface tokens so the chips read as miniature
	// dark/light cards instead of raw black/white. `bg` and `fg` are
	// always opposites — co-locating the mapping keeps the contrast
	// invariant in one place.
	function surfaces(iconBg: string | null | undefined) {
		const wantsDarkSurface = iconBg === '#000000';
		return {
			bg: wantsDarkSurface ? 'var(--surface-dark)' : 'var(--surface-light)',
			fg: wantsDarkSurface ? 'var(--surface-light)' : 'var(--surface-dark)',
		};
	}
</script>

<aside
	data-slot="timeline-rail"
	class="bg-sidebar relative z-30 flex h-full w-12 shrink-0 flex-col"
	aria-label="Recent activity"
>
	<ScrollArea.Root class="min-h-0 flex-1" scrollbarYClasses="w-1.5">
		<Timeline.Root>
			{#each activityService.recent as item, i (item.id)}
				{@const { bg, fg } = surfaces(item.accent?.iconBg)}
				<Timeline.Item
					color={item.accent?.hex}
					iconBg={bg}
					iconColor={fg}
					highlighted={i === 0}
					iconSrc={item.iconBase64}
					name={item.name}
				/>
			{/each}
		</Timeline.Root>
	</ScrollArea.Root>
</aside>
