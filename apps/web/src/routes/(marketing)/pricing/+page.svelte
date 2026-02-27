<script lang="ts">
	import { page } from '$app/state';
	import GetProButton from '$lib/components/GetProButton.svelte';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Select from '@eurora/ui/components/select/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import * as Sheet from '@eurora/ui/components/sheet/index';
	import { Textarea } from '@eurora/ui/components/textarea/index';
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import CheckIcon from '@lucide/svelte/icons/check';
	import MinusIcon from '@lucide/svelte/icons/minus';
	import countries from 'i18n-iso-countries';
	import enLocale from 'i18n-iso-countries/langs/en.json';

	const shouldAutoCheckout = page.url.searchParams.get('checkout') === 'true';

	countries.registerLocale(enLocale);
	const countryNames = Object.values(countries.getNames('en', { select: 'official' })).sort();

	let contactSheetOpen = $state(false);
	let step = $state<1 | 2>(1);
	let companyEmail = $state('');
	let firstName = $state('');
	let lastName = $state('');
	let company = $state('');
	let country = $state('');
	let employees = $state('');
	let details = $state('');

	function resetForm() {
		step = 1;
		companyEmail = '';
		firstName = '';
		lastName = '';
		company = '';
		country = '';
		employees = '';
		details = '';
	}

	function openContactSheet() {
		resetForm();
		contactSheetOpen = true;
	}
</script>

<div class="container mx-auto max-w-5xl px-4 pt-16 pb-24">
	<div class="mb-6">
		<p class="text-sm font-medium tracking-widest uppercase text-primary mb-3">Pricing</p>
		<h1 class="text-4xl font-bold mb-4 sm:text-5xl">Use it free. Pay when it matters.</h1>
		<p class="max-w-2xl text-lg text-muted-foreground leading-relaxed">
			Eurora is free forever for personal use. Pro allows you to run state of the art models
			in the cloud. Enterpise is for companies.
		</p>
	</div>

	<Separator class="mb-16" />

	<div class="grid grid-cols-1 gap-6 mb-20 md:grid-cols-3">
		<div class="rounded-2xl border border-border bg-card p-8 flex flex-col">
			<div class="mb-6">
				<h2 class="text-xl font-bold mb-1">Free</h2>
				<p class="text-sm text-muted-foreground">Forever, no limits. We work for you</p>
			</div>
			<div class="mb-6">
				<span class="text-4xl font-bold">€0</span>
				<span class="text-muted-foreground text-sm">/forever</span>
			</div>
			<ul class="space-y-3 mb-8 flex-1">
				<li class="flex items-center gap-2.5 text-sm">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Unlimited local usage
				</li>
				<li class="flex items-center gap-2.5 text-sm">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Desktop & browser apps
				</li>
				<li class="flex items-center gap-2.5 text-sm">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Bring your own API keys or local models
				</li>
			</ul>
			<Button variant="outline" class="w-full" href="/download">Download</Button>
		</div>

		<div class="group relative">
			<div
				class="absolute -inset-px rounded-2xl bg-gradient-to-b from-primary/30 via-primary/10 to-transparent"
			></div>
			<div
				class="relative rounded-2xl border border-primary/40 bg-card p-8 flex flex-col h-full"
			>
				<div class="flex items-center gap-2 mb-6">
					<div>
						<h2 class="text-xl font-bold mb-1">Pro</h2>
						<p class="text-sm text-muted-foreground">
							Get access to fully private and secure European cloud
						</p>
					</div>
					<Badge class="ml-auto">Popular</Badge>
				</div>
				<div class="mb-2">
					<span class="text-4xl font-bold">€9.99</span>
					<span class="text-muted-foreground text-sm">/first month</span>
				</div>
				<p class="text-sm text-muted-foreground mb-6">
					<span class="line-through">€19.99</span> — 50% off your first month
				</p>
				<ul class="space-y-3 mb-8 flex-1">
					<li class="flex items-center gap-2.5 text-sm">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						Unlimited queries
					</li>
					<li class="flex items-center gap-2.5 text-sm">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						All AI models
					</li>
					<li class="flex items-center gap-2.5 text-sm">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						Priority support
					</li>
					<li class="flex items-center gap-2.5 text-sm">
						<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
						Everything in Free
					</li>
				</ul>
				<GetProButton class="w-full" autoTrigger={shouldAutoCheckout}>Get Pro</GetProButton>
			</div>
		</div>

		<div class="rounded-2xl border border-border bg-card p-8 flex flex-col">
			<div class="mb-6">
				<h2 class="text-xl font-bold mb-1">Enterprise</h2>
				<p class="text-sm text-muted-foreground">For companies of any size</p>
			</div>
			<div class="mb-6">
				<span class="text-4xl font-bold">Custom</span>
			</div>
			<ul class="space-y-3 mb-8 flex-1">
				<li class="flex items-center gap-2.5 text-sm">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					API access & integrations
				</li>
				<li class="flex items-center gap-2.5 text-sm">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Isolated deployment on your own cloud
				</li>
				<li class="flex items-center gap-2.5 text-sm">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Dedicated support
				</li>
				<li class="flex items-center gap-2.5 text-sm">
					<CheckIcon class="h-4 w-4 shrink-0 text-primary" />
					Everything in Pro
				</li>
			</ul>
			<Button variant="outline" class="w-full" onclick={openContactSheet}
				>Contact Sales</Button
			>
		</div>
	</div>
</div>

<Sheet.Root
	bind:open={contactSheetOpen}
	onOpenChange={(open) => {
		if (!open) resetForm();
	}}
>
	<Sheet.Content side="right" class="overflow-y-auto">
		<Sheet.Header>
			<Sheet.Title>Contact Sales</Sheet.Title>
			<Sheet.Description>
				{#if step === 1}
					Enter your company email to get started.
				{:else}
					Tell us about your organization.
				{/if}
			</Sheet.Description>
		</Sheet.Header>

		{#if step === 1}
			<div class="space-y-4 p-4">
				<div class="space-y-2">
					<Label for="company-email">Company Email</Label>
					<Input
						id="company-email"
						type="email"
						placeholder="you@company.com"
						bind:value={companyEmail}
					/>
				</div>
				<Button
					class="w-full"
					disabled={!companyEmail}
					onclick={() => {
						step = 2;
					}}
				>
					Continue
					<ArrowRightIcon class="ml-2 h-4 w-4" />
				</Button>
			</div>
		{:else}
			<div class="space-y-4 p-4">
				<div class="grid grid-cols-2 gap-4">
					<div class="space-y-2">
						<Label for="first-name">First Name</Label>
						<Input id="first-name" placeholder="Jane" bind:value={firstName} />
					</div>
					<div class="space-y-2">
						<Label for="last-name">Last Name</Label>
						<Input id="last-name" placeholder="Doe" bind:value={lastName} />
					</div>
				</div>

				<div class="space-y-2">
					<Label for="company">Company</Label>
					<Input id="company" placeholder="Acme Inc." bind:value={company} />
				</div>

				<div class="space-y-2">
					<Label>Country</Label>
					<Select.Root type="single" bind:value={country}>
						<Select.Trigger>
							{country || 'Select a country'}
						</Select.Trigger>
						<Select.Content>
							{#each countryNames as c}
								<Select.Item value={c}>{c}</Select.Item>
							{/each}
						</Select.Content>
					</Select.Root>
				</div>

				<div class="space-y-2">
					<Label>Employees</Label>
					<Select.Root type="single" bind:value={employees}>
						<Select.Trigger>
							{employees || 'Select company size'}
						</Select.Trigger>
						<Select.Content>
							<Select.Item value="1-100">1-100</Select.Item>
							<Select.Item value="100-750">100-750</Select.Item>
							<Select.Item value="750-5000">750-5,000</Select.Item>
							<Select.Item value="5000+">5,000+</Select.Item>
						</Select.Content>
					</Select.Root>
				</div>

				<div class="space-y-2">
					<Label for="details">Additional Details</Label>
					<Textarea
						id="details"
						placeholder="Tell us about your needs..."
						bind:value={details}
					/>
				</div>
			</div>

			<Sheet.Footer>
				<Button class="w-full">Submit</Button>
			</Sheet.Footer>
		{/if}
	</Sheet.Content>
</Sheet.Root>
