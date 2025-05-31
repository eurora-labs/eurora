<script lang="ts">
	import type { WithElementRef } from 'bits-ui';
	import type { HTMLAttributes } from 'svelte/elements';
	import { cn } from '$lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		children,
		...restProps
	}: WithElementRef<HTMLAttributes<HTMLElement>> = $props();
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
			<div class="group/conversation-turn relative flex w-full min-w-0 flex-col">
				<div class="flex-col gap-1 md:gap-3">
					<div class="flex min-w-[500px] max-w-full flex-grow flex-col">
						<div
							class="text-message relative flex min-h-8 w-full flex-col items-end gap-2 whitespace-normal break-words text-start"
						>
							<div class="w-full">
								<div
									class="flex w-full flex-col items-end gap-1 empty:hidden rtl:items-start"
								>
									<div
										class="relative max-w-[70%] rounded-3xl bg-token-message-surface px-5 py-2.5"
									>
										<div class=" whitespace-pre-wrap">
											{@render children?.()}
										</div>
									</div>
								</div>
							</div>
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
</article>
