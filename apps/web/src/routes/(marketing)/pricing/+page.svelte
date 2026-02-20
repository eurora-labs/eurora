<script lang="ts">
	import GetProButton from '$lib/components/GetProButton.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import XIcon from '@lucide/svelte/icons/x';
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
				<Button variant="outline" class="w-full">Contact Sales</Button>
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
