<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import type { ChangeType } from './package-info-context.svelte.js';
	import { cn } from '$lib/utils.js';
	import { setPackageInfo } from './package-info-context.svelte.js';

	let {
		class: className,
		name,
		currentVersion,
		newVersion,
		changeType,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		name: string;
		currentVersion?: string;
		newVersion?: string;
		changeType?: ChangeType;
		children?: Snippet;
	} = $props();

	setPackageInfo({
		name: () => name,
		currentVersion: () => currentVersion,
		newVersion: () => newVersion,
		changeType: () => changeType,
	});
</script>

<div
	data-slot="package-info"
	class={cn('rounded-lg border bg-background p-4', className)}
	{...restProps}
>
	{@render children?.()}
</div>
