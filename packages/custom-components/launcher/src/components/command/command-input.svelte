<script lang="ts">
	import { Command as CommandPrimitive } from 'bits-ui';
	import { Search } from '@lucide/svelte';
	import { cn } from '@eurora/ui/utils.js';
	import { InputArea } from '../input-area/index.js';

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(''),
		height = $bindable('100px'),
		...restProps
	}: CommandPrimitive.InputProps = $props();
</script>

<div class="items-top flex h-fit border-none px-3" data-command-input-wrapper="">
	<Search class="mr-2 mt-7 shrink-0 opacity-50" size="40" style="color: rgba(0, 0, 0, 0.8); " />
	<CommandPrimitive.Input
		class={cn(
			'custom-input my-[15px] flex w-full rounded-md border-none bg-transparent py-0 shadow-none outline-none focus:border-transparent focus:ring-0 disabled:cursor-not-allowed disabled:opacity-50',
			className
		)}
		style="font-size: 40px; line-height: 70px; box-shadow: none;  color: rgba(0, 0, 0, 0.5); padding: 0px; font-weight: 400; height: 70px;"
		bind:ref
		bind:value
		{...restProps}
	>
		{#snippet child({ props })}
			<InputArea bind:ref bind:value {...props} />
		{/snippet}
	</CommandPrimitive.Input>
</div>
