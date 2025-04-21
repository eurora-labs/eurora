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
	viewBox="0 0 32 32"
>
	{#if title?.id && title.title}
		<title id={title.id}>{title.title}</title>
	{/if}
	{#if desc?.id && desc.desc}
		<desc id={desc.id}>{desc.desc}</desc>
	{/if}
	<defs id="defs11">
		<linearGradient
			id="linearGradient12"
			x1="-7.9197998"
			x2="13.276"
			y1="-22.497999"
			y2="-22.497999"
			gradientTransform="matrix(1.1330982,0,0,1.135195,-1.6568437,3.3675398)"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#9369de" offset="0" id="stop2" style="stop-color:#9f7ae1;stop-opacity:1;" />
			<stop
				stop-color="#5d5de5"
				offset="0.39556"
				id="stop4"
				style="stop-color:#6565e6;stop-opacity:1;"
			/>
			<stop
				stop-color="#3679f2"
				offset="0.80444002"
				id="stop6"
				style="stop-color:#3679f2;stop-opacity:1;"
			/>
			<stop stop-color="#0f89ff" offset="1" id="stop8" style="stop-color:#0f89ff;stop-opacity:1;" />
		</linearGradient>
	</defs>
	<g
		transform="matrix(1.3405063,0,0,1.3459073,-6.1785688,-3.9305535)"
		id="g15"
		style="display:inline;stroke:none;stroke-width:0.372245;stroke-dasharray:none;stroke-opacity:1"
	>
		<ellipse
			transform="matrix(-0.62559098,0.78015122,-0.78507343,-0.61940271,0,0)"
			cx="1.3776516"
			cy="-22.17215"
			rx="12.008981"
			ry="11.813949"
			fill="url(#linearGradient12)"
			style="display:inline;fill:url(#linearGradient12);stroke:none;stroke-width:0.372251;stroke-dasharray:none;stroke-opacity:1;paint-order:stroke fill markers"
			id="ellipse13"
			mask="none"
			clip-path="none"
		/>
	</g>
	<g
		transform="matrix(0.93457371,0,0,0.99714182,1.2590407,-0.24936295)"
		stroke="#ffffff"
		stroke-dashoffset="25.624"
		stroke-linecap="round"
		stroke-linejoin="round"
		id="g23"
		style="stroke:#ffffff;stroke-width:1.55412;stroke-dasharray:none;stroke-opacity:1"
	>
		<path
			d="m 12.751266,1.0269795 7.98907,21.0567415 10.911276,1.399117 z"
			fill="#1d90ff"
			stroke-width="2.18477"
			style="display:inline;fill:none;fill-opacity:1;stroke:#ffffff;stroke-width:1.55412;stroke-dasharray:none;stroke-opacity:1;paint-order:stroke markers fill"
			id="path17"
		/>
		<path
			d="m 12.751266,1.0269795 7.989292,21.0567415 c 0,0 -17.1712447,7.789338 -17.1712447,7.789338 z"
			fill="#cab7fa"
			stroke-width="2.18477"
			style="display:inline;fill:none;fill-opacity:1;stroke:#ffffff;stroke-width:1.55412;stroke-dasharray:none;stroke-opacity:1;paint-order:stroke markers fill"
			id="path19"
		/>
		<path
			d="M 3.8648648,29.814872 20.893294,22.086661 31.713647,23.474788 Z"
			fill="#a01fef"
			stroke-width="2.16709"
			style="display:inline;fill:none;fill-opacity:1;stroke:#ffffff;stroke-width:1.55412;stroke-dasharray:none;stroke-opacity:1;paint-order:stroke markers fill"
			id="path21"
		/>
	</g>
</svg>
