<script lang="ts">
	import { getContext } from 'svelte';
	import type { BaseProps, Props } from './types.js';

	const ctx: BaseProps = getContext('iconCtx') ?? {};

	let {
		size = ctx.size || '24',
		role = ctx.role || 'img',
		color = ctx.color || 'currentColor',
		strokeWidth = ctx.strokeWidth || '0',
		title,
		desc,
		ariaLabel = 'github',
		...restProps
	}: Props = $props();

	let ariaDescribedby = `${title?.id || ''} ${desc?.id || ''}`;
	const hasDescription = $derived(!!(title?.id || desc?.id));
</script>

<svg
	xmlns="http://www.w3.org/2000/svg"
	{...restProps}
	{role}
	width={size}
	height={size}
	fill="none"
	stroke={color}
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-label={ariaLabel}
	preserveAspectRatio="xMidYMid"
	aria-describedby={hasDescription ? ariaDescribedby : undefined}
	viewBox="0 0 18 18"
>
	{#if title?.id && title.title}
		<title id={title.id}>{title.title}</title>
	{/if}
	{#if desc?.id && desc.desc}
		<desc id={desc.id}>{desc.desc}</desc>
	{/if}
	<defs
		><clipPath id="a"><path d="M6.9766 5.4375h19.359v19.531H6.9766z" /></clipPath><clipPath id="b"
			><path d="M6.9766 5.4375h12.023v19.531H6.9766z" /></clipPath
		><clipPath id="c"><path d="M13 5.4375h13.336v15.562H13z" /></clipPath></defs
	><g clip-path="url(#a)" transform="matrix(.9315 0 0 .9216 -6.4985 -5.0112)"
		><path fill="#a020ef" d="m13.328 5.4375 12.973 15.273-19.324 4.3359z" /></g
	><g clip-path="url(#b)" transform="matrix(.9315 0 0 .9216 -6.4985 -5.0112)"
		><path fill="#c9b6fa" d="M18.844 19.645 6.977 25.0473l6.3516-19.609z" /></g
	><g clip-path="url(#c)" transform="matrix(.9315 0 0 .9216 -6.4985 -5.0112)"
		><path fill="#1d90ff" d="m26.301 20.711-7.457-1.0625-5.5156-14.211z" /></g
	>
</svg>
