<script lang="ts">
	import { Katex } from '@eurora/katex';
	import Bot from '@lucide/svelte/icons/bot';

	let { isAgent, text = $bindable(), finishRendering, class: className } = $props();
</script>

<article
	class="text-token-text-primary w-full scroll-mb-[var(--thread-trailing-height,150px)] focus-visible:outline-2 focus-visible:outline-offset-[-4px] {className}"
>
	<div class="m-auto w-full px-3 py-[18px] text-base md:px-4 md:px-5 lg:px-4 xl:px-5">
		<div
			class="mx-auto flex flex-1 gap-4 text-base md:max-w-3xl md:gap-5 lg:max-w-[40rem] lg:gap-6 xl:max-w-[48rem]"
		>
			{#if isAgent}
				<div class="relative flex flex-shrink-0 flex-col items-end">
					<Bot />
				</div>
			{/if}
			<div class="group/conversation-turn relative flex w-full min-w-0 flex-col">
				<div class="flex-col gap-1 md:gap-3">
					<div class="flex max-w-full flex-grow flex-col">
						{#if isAgent}
							<div
								class="text-message flex min-h-8 w-full flex-col items-end gap-2 whitespace-normal break-words text-start [.text-message+&]:mt-5"
							>
								<div class="flex w-full flex-col gap-1 first:pt-[3px] empty:hidden">
									<div class="markdown conversation-text prose w-full break-words">
										<p>
											<Katex bind:math={text} {finishRendering} />
										</p>
									</div>
								</div>
							</div>
						{:else}
							<div
								class="text-message flex min-h-8 w-full flex-col items-end gap-2 whitespace-normal break-words text-start [.text-message+&]:mt-5"
							>
								<div class="flex w-full flex-col items-end gap-1 empty:hidden rtl:items-start">
									<div
										class="bg-token-message-surface conversation-text relative max-w-[var(--user-chat-width,70%)] rounded-3xl px-5 py-2.5"
									>
										<Katex bind:math={text} {finishRendering} />
									</div>
								</div>
							</div>
						{/if}
					</div>
				</div>
			</div>
		</div>
	</div>
</article>
