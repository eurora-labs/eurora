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
	viewBox="0 0 8 8"
>
	{#if title?.id && title.title}
		<title id={title.id}>{title.title}</title>
	{/if}
	{#if desc?.id && desc.desc}
		<desc id={desc.id}>{desc.desc}</desc>
	{/if}
	<defs id="defs27273">
		<!-- Background radial gradient -->
		<linearGradient
			id="linearGradient12"
			x1="-7.9197998"
			x2="13.276"
			y1="-22.497999"
			y2="-22.497999"
			gradientTransform="matrix(1.1330982,0,0,1.135195,-11.923032,-34.521349)"
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
		<!-- Light drop‑shadow for the facets -->
		<filter id="shadow" x="-0.093270048" y="-0.09077815" width="1.1865381" height="1.2023492">
			<feOffset in="SourceAlpha" dy="0.6" result="off" id="feOffset27260" />
			<feGaussianBlur in="off" stdDeviation="0.8" result="blur" id="feGaussianBlur27262" />
			<feColorMatrix
				in="blur"
				type="matrix"
				values="0 0 0 0 0                               0 0 0 0 0                               0 0 0 0 0                               0 0 0 0.25 0"
				result="shadow"
				id="feColorMatrix27264"
			/>
			<feMerge id="feMerge27270">
				<feMergeNode in="shadow" id="feMergeNode27266" />
				<feMergeNode in="SourceGraphic" id="feMergeNode27268" />
			</feMerge>
		</filter>
		<linearGradient
			xlink:href="#linearGradient12"
			id="linearGradient46190"
			x1="-9.2661877"
			y1="-21.888889"
			x2="20.733812"
			y2="-21.888889"
			gradientUnits="userSpaceOnUse"
			gradientTransform="matrix(0.25000001,0,0,0.25000001,0.20982314,0.02830523)"
		/>
	</defs>
	<!-- ─── BACKGROUND ───────────────────────────────────────────────────────── -->
	<circle
		cx="1.6432759"
		cy="-5.4439158"
		r="3.75"
		fill="url(#linearGradient12)"
		id="circle27275"
		style="fill:url(#linearGradient46190);fill-opacity:1;stroke-width:0.25"
		transform="rotate(120.32114)"
	/>
	<!-- ─── FACETS ───────────────────────────────────────────────────────────── -->
	<g
		stroke="#ffffff"
		stroke-width="1.4"
		stroke-linecap="round"
		stroke-linejoin="round"
		fill="none"
		filter="url(#shadow)"
		id="g27283"
		transform="matrix(0.25,0,0,0.25,-0.11787365,0.16683126)"
	>
		<!-- top‑left to top‑right to mid‑right triangle -->
		<path d="m 12.75,1.02 7.99,21.06 10.92,1.4 z" id="path27277" />
		<!-- top‑left to mid‑right to bottom‑left triangle -->
		<path d="m 12.75,1.02 7.99,21.06 -17.17,7.8 z" id="path27279" />
		<!-- bottom‑left to mid‑right to mid‑right‑end triangle -->
		<path d="m 3.57,29.88 17.17,-7.8 10.92,1.4 z" id="path27281" />
	</g>
</svg>
