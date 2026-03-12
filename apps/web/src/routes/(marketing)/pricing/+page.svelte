<script lang="ts">
	import { page } from '$app/state';
	import GetProButton from '$lib/components/GetProButton.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import CopyIcon from '@lucide/svelte/icons/copy';

	const shouldAutoCheckout = page.url.searchParams.get('checkout') === 'true';

	let copiedEmail = $state(false);

	async function copyEmail() {
		await navigator.clipboard.writeText('contact@eurora-labs.com');
		copiedEmail = true;
		setTimeout(() => (copiedEmail = false), 2000);
	}
</script>

<div class="container mx-auto max-w-7xl px-4 pt-16 pb-24">
	<div class="mb-6">
		<p class="text-sm font-medium tracking-widest uppercase text-primary mb-3">Pricing</p>
		<h1 class="text-4xl font-bold mb-4 sm:text-5xl">Use it free. Pay when it matters.</h1>
		<p class="max-w-2xl text-lg text-muted-foreground leading-relaxed">
			Get started with limited free cloud calls every month, or go Pro for unlimited access to
			European cloud AI.
		</p>
	</div>

	<Separator class="mb-16" />

	<div class="grid grid-cols-1 gap-8 mb-20 md:grid-cols-3">
		<div class="rounded-2xl border border-border bg-card p-10 flex flex-col gap-8">
			<h2 class="text-2xl font-bold">Free</h2>
			<p class="text-sm text-muted-foreground">Limited cloud calls every month</p>
			<div>
				<span class="text-5xl font-bold">€0</span>
				<span class="text-muted-foreground text-sm">/month</span>
			</div>
			<Button variant="outline" class="w-full" href="/download">Try Eurora</Button>
			<Separator />
			<ul class="space-y-4 flex-1">
				<li class="flex items-center gap-2.5 text-base">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Limited cloud calls per month
				</li>
				<li class="flex items-center gap-2.5 text-base">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Full browser integration
				</li>
				<li class="flex items-center gap-2.5 text-base">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Unlimited storage
				</li>
				<li class="flex items-center gap-2.5 text-base">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					<span
						><a
							href="/docs/self-hosting"
							class="underline underline-offset-2 hover:text-primary">Self-host</a
						> to use with local models</span
					>
				</li>
			</ul>
		</div>

		<div class="group relative">
			<div
				class="absolute -inset-px rounded-2xl bg-linear-to-b from-primary/30 via-primary/10 to-transparent"
			></div>
			<div
				class="relative rounded-2xl border border-primary/40 bg-card p-10 flex flex-col gap-8 h-full"
			>
				<div class="flex items-center justify-between">
					<h2 class="text-2xl font-bold">Pro</h2>
					<span class="text-2xl text-orange-500">Recommended</span>
				</div>
				<p class="text-sm text-muted-foreground">
					Fully private and secure European cloud AI
				</p>
				<div>
					<span class="text-5xl font-bold">€19.99</span>
					<span class="text-muted-foreground text-sm">/month</span>
				</div>
				<GetProButton class="w-full" autoTrigger={shouldAutoCheckout}
					>Try Eurora</GetProButton
				>
				<Separator />
				<ul class="space-y-4 flex-1">
					<li class="flex items-center gap-2.5 text-base">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						Everything in Free
					</li>
					<li class="flex items-center gap-2.5 text-base">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						Unlimited queries
					</li>
					<li class="flex items-center gap-2.5 text-base">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						Priority support
					</li>
					<li class="flex items-center gap-2.5 text-base">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						All data stored in a Sovereign European Data Center
					</li>
					<li class="flex items-center gap-2.5 text-base">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						No identifiable logs, fully private data storage
					</li>
				</ul>
			</div>
		</div>

		<div class="rounded-2xl border border-border bg-card p-10 flex flex-col gap-8">
			<h2 class="text-2xl font-bold">Enterprise</h2>
			<p class="text-sm text-muted-foreground">For companies of any size</p>
			<div>
				<span class="text-5xl font-bold">Custom</span>
			</div>
			<Button variant="outline" class="w-full" onclick={copyEmail}>
				contact@eurora-labs.com
				<CopyIcon class="h-3.5 w-3.5 text-muted-foreground" />
				{#if copiedEmail}
					<span class="text-xs text-primary">Copied!</span>
				{/if}</Button
			>
			<Separator />
			<ul class="space-y-4 flex-1">
				<li class="flex items-center gap-2.5 text-base">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Everything in Pro
				</li>
				<li class="flex items-center gap-2.5 text-base">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					API access &amp; integrations
				</li>
				<li class="flex items-center gap-2.5 text-base">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Isolated deployment on your own cloud
				</li>
				<li class="flex items-center gap-2.5 text-base">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Dedicated support
				</li>
			</ul>
		</div>
	</div>
</div>
