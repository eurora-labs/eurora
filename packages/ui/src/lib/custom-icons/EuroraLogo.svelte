<script lang="ts">
	import { getContext } from 'svelte';
	import type { BaseProps, Props } from '$lib/custom-icons/types.js';

	const ctx: BaseProps = getContext('iconCtx') ?? {};

	let {
		size = ctx.size || '24',
		role = ctx.role || 'img',
		strokeWidth = ctx.strokeWidth || '0',
		title,
		desc,
		ariaLabel = 'github',
		...restProps
	}: Props = $props();

	const ariaDescribedby = $derived(`${title?.id || ''} ${desc?.id || ''}`);
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
			gradientTransform="matrix(1.1331,0,0,1.1352,-1.2995,3.0828)"
			gradientUnits="userSpaceOnUse"
		>
			<stop stop-color="#9369de" offset="0" id="stop2" />
			<stop stop-color="#5d5de5" offset=".39556" id="stop4" />
			<stop stop-color="#3679f2" offset=".80444" id="stop6" />
			<stop
				stop-color="#0f89ff"
				offset="1"
				id="stop8"
				style="stop-color:#0f89ff;stop-opacity:1;"
			/>
		</linearGradient>
	</defs>
	<g transform="matrix(1.3406,0,0,1.3459,-6.1616,-4.5108)" id="g15">
		<ellipse
			transform="matrix(-0.62559,0.78015,-0.78507,-0.6194,0,0)"
			cx="1.735"
			cy="-22.457001"
			rx="12.009"
			ry="11.814"
			fill="url(#linearGradient12)"
			style="fill:url(#linearGradient12);paint-order:stroke fill markers"
			id="ellipse13"
		/>
	</g>
	<g
		transform="matrix(1.032,0,0,1.0158,-0.55783,-0.33903)"
		stroke="#ffffff"
		stroke-dashoffset="25.624"
		stroke-linecap="round"
		stroke-linejoin="round"
		id="g23"
	>
		<path
			d="m 13.856199,1.7584077 6.487083,18.3028393 8.859899,1.216133 z"
			fill="#1d90ff"
			stroke-width="1.83546"
			style="paint-order:stroke markers fill"
			id="path17"
		/>
		<path
			d="m 13.804953,1.8109616 6.303161,18.5186894 c 0,0 -13.5472744,6.850459 -13.5472744,6.850459 z"
			fill="#cab7fa"
			stroke-width="1.81987"
			style="paint-order:stroke markers fill"
			id="path19"
		/>
		<path
			d="m 6.793392,27.120689 13.708261,-6.913534 8.710624,1.241796 z"
			fill="#a01fef"
			stroke-width="1.83905"
			style="paint-order:stroke markers fill"
			id="path21"
		/>
	</g></svg
>
