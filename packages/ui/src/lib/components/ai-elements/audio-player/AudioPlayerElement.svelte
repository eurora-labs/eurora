<script lang="ts">
	import type { HTMLAudioAttributes } from 'svelte/elements';

	type Props = HTMLAudioAttributes &
		(
			| { data: { mediaType: string; base64: string }; src?: never }
			| { src: string; data?: never }
		);

	let { src, data, ...restProps }: Props = $props();

	let audioSrc = $derived(
		src ? src : data ? `data:${data.mediaType};base64,${data.base64}` : undefined,
	);

	let audioEl: HTMLAudioElement;

	$effect(() => {
		audioEl?.setAttribute('slot', 'media');
	});
</script>

<audio bind:this={audioEl} data-slot="audio-player-element" src={audioSrc} {...restProps}></audio>
