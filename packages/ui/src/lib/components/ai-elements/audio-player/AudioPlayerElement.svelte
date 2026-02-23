<script lang="ts">
	import type { HTMLAudioAttributes } from 'svelte/elements';

	type Props = HTMLAudioAttributes &
		(
			| { data: { mediaType: string; base64: string }; src?: never }
			| { src: string; data?: never }
		);

	let { src, data, ...restProps }: Props = $props();

	let audioSrc = $derived(src ? src : data ? `data:${data.mediaType};base64,${data.base64}` : undefined);
</script>

<audio data-slot="audio-player-element" slot="media" src={audioSrc} {...restProps}></audio>
