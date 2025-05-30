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
	<defs id="defs1"
		><linearGradient id="linearGradient12"
			><stop style="stop-color:#9970e6;stop-opacity:0;" offset="0" id="stop13" /><stop
				style="stop-color:#e738dd;stop-opacity:0.79911375;"
				offset="1"
				id="stop12"
			/></linearGradient
		><linearGradient id="linearGradient9"
			><stop style="stop-color:#36a9f5;stop-opacity:0.80206794;" offset="0" id="stop9" /><stop
				style="stop-color:#9970e6;stop-opacity:0;"
				offset="1"
				id="stop10"
			/></linearGradient
		><linearGradient
			xlink:href="#linearGradient9"
			id="linearGradient10"
			x1="337.48911"
			y1="372.29364"
			x2="149.59541"
			y2="282.32697"
			gradientUnits="userSpaceOnUse"
			gradientTransform="matrix(1.2785705,0.08486395,-0.06800725,1.0805394,-86.826776,10.489876)"
		/><linearGradient
			xlink:href="#linearGradient12"
			id="linearGradient13"
			x1="334.49734"
			y1="356.34833"
			x2="439.75201"
			y2="207.37177"
			gradientUnits="userSpaceOnUse"
			gradientTransform="matrix(1.207665,0,0,1.0949273,-120.17935,38.128955)"
		/></defs
	><g id="layer1"
		><g id="g14" transform="matrix(1.2083681,0,0,1.176677,-53.516241,-47.718627)"
			><path
				style="display:inline;fill:#9970e6;fill-opacity:1;stroke-width:0.469472;stroke-linecap:round;stroke-linejoin:round;stroke-dashoffset:25.6236;paint-order:fill markers stroke"
				d="m 426.42625,345.993 -102.38513,93.25058 c -4.43847,4.04248 0.40352,0.66576 -5.51741,-0.75657 L 53.421995,374.8039 c -4.537056,-1.0899 -3.59503,2.25121 -1.146518,-1.31609 L 252.33665,73.605012 c 2.32896,-3.393154 -0.61375,-3.310941 1.85246,0.0037 L 426.76009,344.8249 c 2.89362,3.88911 -0.33384,-4.04207 -0.33384,1.16789 z"
				id="path1"
			/><path
				style="display:inline;fill:url(#linearGradient10);stroke-width:0.471138;stroke-linecap:round;stroke-linejoin:round;stroke-dashoffset:25.6236;paint-order:fill markers stroke"
				d="M 321.08595,439.95414 53.391215,375.39497 c -4.699483,-1.22908 -2.773503,3.34388 0,0 L 254.22474,70.548094 c 1.75109,-2.11122 -3.93683,20.576094 1.05694,1.458915 l 67.35433,363.987121 c 0.35089,2.67002 1.49544,4.75651 -1.55006,3.96001 z"
				id="path5"
			/><path
				style="display:inline;fill:url(#linearGradient13);stroke-width:0.459966;stroke-linecap:round;stroke-linejoin:round;stroke-dashoffset:25.6236;paint-order:fill markers stroke"
				d="M 427.4126,345.34072 324.36288,442.06847 253.53844,75.183022 c -0.3047,-1.66732 -0.34383,-1.827716 0.68898,-0.420786 L 427.38178,344.14312 c 2.73617,3.72739 0.40146,-4.24229 0.0676,0.65722 z"
				id="path7"
			/></g
		></g
	>
</svg>
