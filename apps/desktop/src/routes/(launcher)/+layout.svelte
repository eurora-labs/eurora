<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { onMount } from 'svelte';
	import '../../app.pcss';
	let { children } = $props();
	let mainRef = $state<HTMLElement | null>(null);
	onMount(() => {
		invoke('get_scale_factor', { height: mainRef?.scrollHeight }).then(async (result) => {
			const scaleFactor = result as number;

			const resizeObserver = new ResizeObserver(() => {
				if (!mainRef) return;
				try {
					console.log('scrollHeight', mainRef?.scrollHeight);
					console.log('offsetHeight', mainRef?.offsetHeight);
					console.log('clientHeight', mainRef?.clientHeight);
					invoke('resize_window', {
						height: mainRef.scrollHeight,
						scaleFactor: scaleFactor
					});
				} catch (error) {
					console.error('Failed to resize window:', error);
				}
			});

onMount(() => {
  let resizeObserver: ResizeObserver | undefined;
  
  invoke('get_scale_factor', { height: mainRef?.scrollHeight }).then(async (result) => {
    const scaleFactor = result as number;

    resizeObserver = new ResizeObserver(() => {
      try {
        invoke('resize_window', {
          height: mainRef?.scrollHeight,
          scaleFactor
        });
      } catch (error) {
        console.error('Failed to resize window:', error);
      }
    });

    resizeObserver.observe(mainRef!);
  });
  
  return () => {
    if (resizeObserver) {
      resizeObserver.disconnect();
    }
  };
});
		});
	});
</script>

<main bind:this={mainRef} class="h-fit min-h-[100px] flex-1 bg-transparent">
	{@render children?.()}
</main>
