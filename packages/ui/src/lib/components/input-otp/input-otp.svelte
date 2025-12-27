<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { PinInput as InputOTPPrimitive } from 'bits-ui';

	let {
		ref = $bindable(null),
		class: className,
		value = $bindable(''),
		...restProps
	}: InputOTPPrimitive.RootProps = $props();
</script>

<InputOTPPrimitive.Root
	bind:ref
	bind:value
	data-slot="input-otp"
	class={cn(
		'has-disabled:opacity-50 flex items-center gap-2 [&_input]:disabled:cursor-not-allowed',
		className,
	)}
	{...restProps}
/>
