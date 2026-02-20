<script lang="ts">
	import GetProButton from '$lib/components/GetProButton.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as Select from '@eurora/ui/components/select/index';
	import * as Sheet from '@eurora/ui/components/sheet/index';
	import { Textarea } from '@eurora/ui/components/textarea/index';
	import ArrowRightIcon from '@lucide/svelte/icons/arrow-right';
	import CheckIcon from '@lucide/svelte/icons/check';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import XIcon from '@lucide/svelte/icons/x';
	import countries from 'i18n-iso-countries';
	import enLocale from 'i18n-iso-countries/langs/en.json';

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

<div class="container mx-auto max-w-5xl px-4 py-16">
	<div class="mb-12 text-center">
		<h1 class="mb-4 text-4xl font-bold">Simple, Transparent Pricing</h1>
		<p class="mx-auto max-w-2xl text-xl text-gray-600">
			Choose the plan that works best for you. All plans include core features with different
			usage limits.
		</p>
	</div>

	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-3">
		<Card.Root class="border-t-4 border-gray-400 p-6">
			<Card.Header>
				<Card.Title>Free</Card.Title>
				<Card.Description>Perfect for casual users</Card.Description>
				<div class="mt-4">
					<span class="text-4xl font-bold">$0</span>
					<span class="text-gray-600">/month</span>
				</div>
			</Card.Header>
			<Card.Content>
				<ul class="mb-6 space-y-3">
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Basic AI assistance</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Up to 20 queries per day</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Standard response time</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Desktop and mobile apps</span>
					</li>
					<li class="flex items-start">
						<XIcon class="mr-2 mt-0.5 h-5 w-5 text-gray-400" />
						<span class="text-gray-500">Advanced AI models</span>
					</li>
					<li class="flex items-start">
						<XIcon class="mr-2 mt-0.5 h-5 w-5 text-gray-400" />
						<span class="text-gray-500">Priority support</span>
					</li>
					<li class="flex items-start">
						<XIcon class="mr-2 mt-0.5 h-5 w-5 text-gray-400" />
						<span class="text-gray-500">Custom integrations</span>
					</li>
				</ul>
				<Button variant="outline" class="w-full" href="/register">Get Started</Button>
			</Card.Content>
		</Card.Root>

		<Card.Root class="relative border-t-4 border-purple-600 p-6 shadow-lg">
			<div
				class="absolute right-0 top-0 rounded-bl-md bg-purple-600 px-3 py-1 text-sm font-medium text-white"
			>
				Popular
			</div>
			<Card.Header>
				<Card.Title>Pro</Card.Title>
				<Card.Description>For power users and professionals</Card.Description>
				<div class="mt-4">
					<span class="text-4xl font-bold">$20</span>
					<span class="text-gray-600">/month</span>
				</div>
			</Card.Header>
			<Card.Content>
				<ul class="mb-6 space-y-3">
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Advanced AI assistance</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Unlimited queries</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Faster response time</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Desktop and mobile apps</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Access to premium AI models</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Priority email support</span>
					</li>
					<li class="flex items-start">
						<XIcon class="mr-2 mt-0.5 h-5 w-5 text-gray-400" />
						<span class="text-gray-500">Custom integrations</span>
					</li>
				</ul>
				<GetProButton class="w-full">Get</GetProButton>
			</Card.Content>
		</Card.Root>

		<Card.Root class="border-t-4 border-blue-600 p-6">
			<Card.Header>
				<Card.Title>Enterprise</Card.Title>
				<Card.Description>For teams and organizations</Card.Description>
				<div class="mt-4">
					<span class="text-4xl font-bold">$29</span>
					<span class="text-gray-600">/user/month</span>
				</div>
			</Card.Header>
			<Card.Content>
				<ul class="mb-6 space-y-3">
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Enterprise-grade AI assistance</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Unlimited queries with higher rate limits</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Fastest response time</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>All apps and platforms</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Access to all AI models</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>24/7 dedicated support</span>
					</li>
					<li class="flex items-start">
						<CheckIcon class="mr-2 mt-0.5 h-5 w-5 text-green-500" />
						<span>Custom integrations & API access</span>
					</li>
				</ul>
				<Button variant="outline" class="w-full" onclick={openContactSheet}
					>Contact Sales</Button
				>
			</Card.Content>
		</Card.Root>
	</div>

	<Card.Root class="mb-16 p-6">
		<Card.Header>
			<Card.Title>Compare Plans</Card.Title>
			<Card.Description>Detailed feature comparison across all plans</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="overflow-x-auto">
				<table class="w-full">
					<thead>
						<tr class="border-b">
							<th class="px-4 py-3 text-left">Feature</th>
							<th class="px-4 py-3 text-center">Free</th>
							<th class="px-4 py-3 text-center">Pro</th>
							<th class="px-4 py-3 text-center">Enterprise</th>
						</tr>
					</thead>
					<tbody>
						<tr class="border-b">
							<td class="px-4 py-3 font-medium">Daily Query Limit</td>
							<td class="px-4 py-3 text-center">20</td>
							<td class="px-4 py-3 text-center">Unlimited</td>
							<td class="px-4 py-3 text-center">Unlimited</td>
						</tr>
						<tr class="border-b">
							<td class="px-4 py-3 font-medium">AI Models</td>
							<td class="px-4 py-3 text-center">Basic</td>
							<td class="px-4 py-3 text-center">Premium</td>
							<td class="px-4 py-3 text-center">All Models</td>
						</tr>
						<tr class="border-b">
							<td class="px-4 py-3 font-medium">Response Time</td>
							<td class="px-4 py-3 text-center">Standard</td>
							<td class="px-4 py-3 text-center">Fast</td>
							<td class="px-4 py-3 text-center">Fastest</td>
						</tr>
						<tr class="border-b">
							<td class="px-4 py-3 font-medium">History Retention</td>
							<td class="px-4 py-3 text-center">7 days</td>
							<td class="px-4 py-3 text-center">90 days</td>
							<td class="px-4 py-3 text-center">Unlimited</td>
						</tr>
						<tr class="border-b">
							<td class="px-4 py-3 font-medium">File Upload Size</td>
							<td class="px-4 py-3 text-center">5MB</td>
							<td class="px-4 py-3 text-center">50MB</td>
							<td class="px-4 py-3 text-center">500MB</td>
						</tr>
						<tr class="border-b">
							<td class="px-4 py-3 font-medium">Team Collaboration</td>
							<td class="px-4 py-3 text-center">
								<XIcon class="mx-auto h-5 w-5 text-gray-400" />
							</td>
							<td class="px-4 py-3 text-center">
								<XIcon class="mx-auto h-5 w-5 text-gray-400" />
							</td>
							<td class="px-4 py-3 text-center">
								<CheckIcon class="mx-auto h-5 w-5 text-green-500" />
							</td>
						</tr>
						<tr class="border-b">
							<td class="px-4 py-3 font-medium">API Access</td>
							<td class="px-4 py-3 text-center">
								<XIcon class="mx-auto h-5 w-5 text-gray-400" />
							</td>
							<td class="px-4 py-3 text-center">Limited</td>
							<td class="px-4 py-3 text-center">Full Access</td>
						</tr>
						<tr class="border-b">
							<td class="px-4 py-3 font-medium">Support</td>
							<td class="px-4 py-3 text-center">Community</td>
							<td class="px-4 py-3 text-center">Email Priority</td>
							<td class="px-4 py-3 text-center">24/7 Dedicated</td>
						</tr>
					</tbody>
				</table>
			</div>
		</Card.Content>
	</Card.Root>

	<div class="grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<SparklesIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Frequently Asked Questions</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="mb-1 text-lg font-medium">Can I change plans later?</h3>
						<p class="text-gray-600">
							Yes, you can upgrade or downgrade your plan at any time. Changes take
							effect at the start of your next billing cycle.
						</p>
					</div>
					<div>
						<h3 class="mb-1 text-lg font-medium">
							Is there a free trial for paid plans?
						</h3>
						<p class="text-gray-600">
							Yes, both Pro and Enterprise plans come with a 14-day free trial. No
							credit card required to start.
						</p>
					</div>
					<div>
						<h3 class="mb-1 text-lg font-medium">
							What payment methods do you accept?
						</h3>
						<p class="text-gray-600">
							We accept all major credit cards, PayPal, and for Enterprise customers,
							we also offer invoicing.
						</p>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Need Help Choosing?</Card.Title>
				<Card.Description>Our team is here to help you find the right plan</Card.Description
				>
			</Card.Header>
			<Card.Content>
				<p class="mb-6 text-gray-600">
					Not sure which plan is right for you? Our team can help you assess your needs
					and recommend the best option for your use case.
				</p>
				<div class="space-y-4">
					<Button variant="outline" class="w-full">Schedule a Demo</Button>
					<Button variant="outline" class="w-full">Contact Sales</Button>
					<p class="mt-2 text-center text-sm text-gray-500">
						Or email us at <span class="text-purple-600">sales@eurora-labs.com</span>
					</p>
				</div>
			</Card.Content>
		</Card.Root>
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
