<script lang="ts">
	import type { WithElementRef } from 'bits-ui';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '@eurora/ui/utils.js';

	import { Katex } from '@eurora/katex';

	let {
		ref = $bindable(null),
		class: className,
		text = $bindable(),
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLElement>> & { text: string } = $props();
</script>

<article
	bind:this={ref}
	class={cn(
		'text-token-text-primary w-full focus-visible:outline-2 focus-visible:outline-offset-[-4px]',
		className
	)}
	{...restProps}
>
	<div class="mx-auto my-auto py-6 text-base">
		<div
			class="mx-auto flex flex-1 gap-4 text-base md:max-w-3xl md:gap-5 lg:max-w-[4rem] lg:gap-6 xl:max-w-[48rem]"
		>
			<div class="group/conversation-turn agent-turn relative flex w-full min-w-0 flex-col px-4">
				<div class="flex-col gap-1 md:gap-3">
					<div class="flex max-w-full flex-grow flex-col">
						<div
							class="text-message relative flex min-h-8 w-full flex-col items-end gap-2 whitespace-normal break-words text-start [.text-message+&]:mt-5"
						>
							<div class="flex w-full flex-col gap-1 first:pt-[3px] empty:hidden">
								<Katex math={text} finishRendering={() => {}} />
								<!-- {text} -->
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</article>
