<script lang="ts">
	import { cn } from '$lib/utils.js';
	import type { HTMLImgAttributes } from 'svelte/elements';
	import qwen from './logos/qwen.svg';
	import qwenCn from './logos/qwen-cn.svg';
	import zai from './logos/zai.svg';

	const logos: Record<string, string> = {
		qwen,
		'qwen-cn': qwenCn,
		zai,
	};

	type Provider = keyof typeof logos | (string & {});

	type Props = Omit<HTMLImgAttributes, 'src' | 'alt'> & {
		provider: Provider;
	};

	let { provider, class: className, ...restProps }: Props = $props();
</script>

<img
	{...restProps}
	data-slot="model-selector-logo"
	alt={`${provider} logo`}
	class={cn('size-3 dark:invert', className)}
	height={12}
	src={logos[provider]}
	width={12}
/>
