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
	stroke-width={strokeWidth}
	stroke-linecap="round"
	stroke-linejoin="round"
	aria-label={ariaLabel}
	preserveAspectRatio="xMidYMid"
	aria-describedby={hasDescription ? ariaDescribedby : undefined}
	viewBox="0 0 512 512"
>
	{#if title?.id && title.title}
		<title id={title.id}>{title.title}</title>
	{/if}
	{#if desc?.id && desc.desc}
		<desc id={desc.id}>{desc.desc}</desc>
	{/if}
	<defs id="defs1">
		<linearGradient id="linearGradient15">
			<stop style="stop-color:#9970e6;stop-opacity:0.57938147;" offset="0" id="stop14" />
			<stop style="stop-color:#e738dd;stop-opacity:0.5;" offset="1" id="stop15" />
		</linearGradient>
		<linearGradient id="linearGradient5">
			<stop style="stop-color:#dc30f7;stop-opacity:0.50515461;" offset="0" id="stop4" />
			<stop style="stop-color:#52a0ee;stop-opacity:0.59381443;" offset="1" id="stop5" />
		</linearGradient>
		<linearGradient id="linearGradient9">
			<stop style="stop-color:#36a9f5;stop-opacity:0.5;" offset="0" id="stop9" />
			<stop style="stop-color:#9970e6;stop-opacity:0.42474228;" offset="1" id="stop10" />
		</linearGradient>
		<linearGradient
			xlink:href="#linearGradient9"
			id="linearGradient10"
			x1="334.74426"
			y1="359.89102"
			x2="119.11613"
			y2="266.39267"
			gradientUnits="userSpaceOnUse"
			gradientTransform="matrix(1.0060888,0,0,1.0060888,-29.406558,69.078557)"
		/>
		<linearGradient
			xlink:href="#linearGradient15"
			id="linearGradient13"
			x1="251.57881"
			y1="245.73819"
			x2="438.79297"
			y2="214.00473"
			gradientUnits="userSpaceOnUse"
			gradientTransform="matrix(1.0060888,0,0,1.0060888,-10.331328,59.7579)"
		/>
		<linearGradient
			xlink:href="#linearGradient5"
			id="linearGradient4"
			gradientUnits="userSpaceOnUse"
			x1="98.433174"
			y1="375.04749"
			x2="444.45438"
			y2="352.66278"
		/>
	</defs>
	<g id="layer1" transform="translate(-9.5892468,-21.554672)">
		<g id="g14" transform="matrix(1.2083681,0,0,1.176677,-53.516241,-47.718627)">
			<path
				style="display:inline;fill:url(#linearGradient4);stroke-width:0.402436;stroke-linecap:round;stroke-linejoin:round;stroke-dashoffset:25.6236;paint-order:fill markers stroke"
				d="M 305.43391,423.91639 109.16454,379.01614 a 3.6067474,3.6067474 84.99064 0 1 -0.59939,-6.83829 l 118.49888,-50.06675 197.02579,30.67921 a 3.4440654,3.4440654 78.587655 0 1 1.27859,6.33409 l -99.53975,61.41758 a 27.261658,27.261658 170.60525 0 1 -20.39475,3.37441 z"
				id="path3"
				transform="translate(-2.314633,19.220323)"
			/>
			<path
				style="display:inline;fill:#ffffff;fill-opacity:1;stroke:#dc6fe6;stroke-width:5.73038;stroke-linecap:round;stroke-linejoin:round;stroke-dasharray:none;stroke-dashoffset:25.6236;stroke-opacity:1;paint-order:stroke fill markers"
				d="m 401.61114,373.31793 -79.16407,47.16213 a 36.439456,36.439456 171.62919 0 1 -27.492,4.04536 L 125.23521,382.07473 a 10.923615,10.923615 68.032568 0 1 -6.61092,-16.3894 L 256.11037,145.85214 a 9.5611651,9.5611651 179.55927 0 1 16.13375,-0.12411 l 134.11435,207.27925 a 14.483513,14.483513 103.15589 0 1 -4.74733,20.31065 z"
				id="path1"
				transform="matrix(0.8737436,0,0,0.87717325,35.543952,43.548349)"
			/>
			<path
				style="display:inline;fill:url(#linearGradient10);stroke-width:0.402436;stroke-linecap:round;stroke-linejoin:round;stroke-dashoffset:25.6236;paint-order:fill markers stroke"
				d="M 93.020475,375.70562 255.27318,131.02474 a 4.7402307,4.7402307 12.398962 0 1 8.63561,1.8985 l 45.02168,292.46922 a 8.3129142,8.3129142 137.09348 0 1 -10.0774,9.36663 L 98.190598,388.66047 a 8.4823932,8.4823932 68.243664 0 1 -5.170123,-12.95485 z"
				id="path5"
			/>
			<path
				style="display:inline;fill:url(#linearGradient13);stroke-width:0.402436;stroke-linecap:round;stroke-linejoin:round;stroke-dashoffset:25.6236;paint-order:fill markers stroke"
				d="M 426.58477,366.29671 324.94236,429.4731 a 6.5384774,6.5384774 24.424993 0 1 -9.9044,-4.49805 L 267.42969,133.82732 a 4.4419633,4.4419633 158.05615 0 1 8.04004,-3.23923 l 153.78116,222.91082 a 9.0305475,9.0305475 101.76792 0 1 -2.66612,12.7978 z"
				id="path7"
			/>
		</g>
	</g>
</svg>
