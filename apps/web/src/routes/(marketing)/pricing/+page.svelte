<script lang="ts">
	import { Card, Button } from '@eurora/ui';
	import { Check, X, Sparkles } from '@lucide/svelte';
</script>

<div class="container mx-auto px-4 py-16 max-w-5xl">
	<div class="text-center mb-12">
		<h1 class="text-4xl font-bold mb-4">Simple, Transparent Pricing</h1>
		<p class="text-xl text-gray-600 max-w-2xl mx-auto">
			Choose the plan that works best for you. All plans include core features with different usage
			limits.
		</p>
	</div>

	<div class="grid grid-cols-1 md:grid-cols-3 gap-8 mb-16">
		<!-- Free Plan -->
		<Card.Root class="p-6 border-t-4 border-gray-400">
			<Card.Header>
				<Card.Title>Free</Card.Title>
				<Card.Description>Perfect for casual users</Card.Description>
				<div class="mt-4">
					<span class="text-4xl font-bold">$0</span>
					<span class="text-gray-600">/month</span>
				</div>
			</Card.Header>
			<Card.Content>
				<ul class="space-y-3 mb-6">
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Basic AI assistance</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Up to 20 queries per day</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Standard response time</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Desktop and mobile apps</span>
					</li>
					<li class="flex items-start">
						<X class="w-5 h-5 text-gray-400 mr-2 mt-0.5" />
						<span class="text-gray-500">Advanced AI models</span>
					</li>
					<li class="flex items-start">
						<X class="w-5 h-5 text-gray-400 mr-2 mt-0.5" />
						<span class="text-gray-500">Priority support</span>
					</li>
					<li class="flex items-start">
						<X class="w-5 h-5 text-gray-400 mr-2 mt-0.5" />
						<span class="text-gray-500">Custom integrations</span>
					</li>
				</ul>
				<Button variant="outline" class="w-full">Get Started</Button>
			</Card.Content>
		</Card.Root>

		<!-- Pro Plan -->
		<Card.Root class="p-6 border-t-4 border-purple-600 shadow-lg relative">
			<div
				class="absolute top-0 right-0 bg-purple-600 text-white px-3 py-1 text-sm font-medium rounded-bl-md"
			>
				Popular
			</div>
			<Card.Header>
				<Card.Title>Pro</Card.Title>
				<Card.Description>For power users and professionals</Card.Description>
				<div class="mt-4">
					<span class="text-4xl font-bold">$12</span>
					<span class="text-gray-600">/month</span>
				</div>
			</Card.Header>
			<Card.Content>
				<ul class="space-y-3 mb-6">
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Advanced AI assistance</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Unlimited queries</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Faster response time</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Desktop and mobile apps</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Access to premium AI models</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Priority email support</span>
					</li>
					<li class="flex items-start">
						<X class="w-5 h-5 text-gray-400 mr-2 mt-0.5" />
						<span class="text-gray-500">Custom integrations</span>
					</li>
				</ul>
				<Button class="w-full">Subscribe Now</Button>
			</Card.Content>
		</Card.Root>

		<!-- Enterprise Plan -->
		<Card.Root class="p-6 border-t-4 border-blue-600">
			<Card.Header>
				<Card.Title>Enterprise</Card.Title>
				<Card.Description>For teams and organizations</Card.Description>
				<div class="mt-4">
					<span class="text-4xl font-bold">$29</span>
					<span class="text-gray-600">/user/month</span>
				</div>
			</Card.Header>
			<Card.Content>
				<ul class="space-y-3 mb-6">
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Enterprise-grade AI assistance</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Unlimited queries with higher rate limits</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Fastest response time</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>All apps and platforms</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Access to all AI models</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>24/7 dedicated support</span>
					</li>
					<li class="flex items-start">
						<Check class="w-5 h-5 text-green-500 mr-2 mt-0.5" />
						<span>Custom integrations & API access</span>
					</li>
				</ul>
				<Button variant="outline" class="w-full">Contact Sales</Button>
			</Card.Content>
		</Card.Root>
	</div>

	<Card.Root class="p-6 mb-16">
		<Card.Header>
			<Card.Title>Compare Plans</Card.Title>
			<Card.Description>Detailed feature comparison across all plans</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="overflow-x-auto">
				<table class="w-full">
					<thead>
						<tr class="border-b">
							<th class="text-left py-3 px-4">Feature</th>
							<th class="text-center py-3 px-4">Free</th>
							<th class="text-center py-3 px-4">Pro</th>
							<th class="text-center py-3 px-4">Enterprise</th>
						</tr>
					</thead>
					<tbody>
						<tr class="border-b">
							<td class="py-3 px-4 font-medium">Daily Query Limit</td>
							<td class="text-center py-3 px-4">20</td>
							<td class="text-center py-3 px-4">Unlimited</td>
							<td class="text-center py-3 px-4">Unlimited</td>
						</tr>
						<tr class="border-b">
							<td class="py-3 px-4 font-medium">AI Models</td>
							<td class="text-center py-3 px-4">Basic</td>
							<td class="text-center py-3 px-4">Premium</td>
							<td class="text-center py-3 px-4">All Models</td>
						</tr>
						<tr class="border-b">
							<td class="py-3 px-4 font-medium">Response Time</td>
							<td class="text-center py-3 px-4">Standard</td>
							<td class="text-center py-3 px-4">Fast</td>
							<td class="text-center py-3 px-4">Fastest</td>
						</tr>
						<tr class="border-b">
							<td class="py-3 px-4 font-medium">History Retention</td>
							<td class="text-center py-3 px-4">7 days</td>
							<td class="text-center py-3 px-4">90 days</td>
							<td class="text-center py-3 px-4">Unlimited</td>
						</tr>
						<tr class="border-b">
							<td class="py-3 px-4 font-medium">File Upload Size</td>
							<td class="text-center py-3 px-4">5MB</td>
							<td class="text-center py-3 px-4">50MB</td>
							<td class="text-center py-3 px-4">500MB</td>
						</tr>
						<tr class="border-b">
							<td class="py-3 px-4 font-medium">Team Collaboration</td>
							<td class="text-center py-3 px-4">
								<X class="w-5 h-5 text-gray-400 mx-auto" />
							</td>
							<td class="text-center py-3 px-4">
								<X class="w-5 h-5 text-gray-400 mx-auto" />
							</td>
							<td class="text-center py-3 px-4">
								<Check class="w-5 h-5 text-green-500 mx-auto" />
							</td>
						</tr>
						<tr class="border-b">
							<td class="py-3 px-4 font-medium">API Access</td>
							<td class="text-center py-3 px-4">
								<X class="w-5 h-5 text-gray-400 mx-auto" />
							</td>
							<td class="text-center py-3 px-4">Limited</td>
							<td class="text-center py-3 px-4">Full Access</td>
						</tr>
						<tr class="border-b">
							<td class="py-3 px-4 font-medium">Support</td>
							<td class="text-center py-3 px-4">Community</td>
							<td class="text-center py-3 px-4">Email Priority</td>
							<td class="text-center py-3 px-4">24/7 Dedicated</td>
						</tr>
					</tbody>
				</table>
			</div>
		</Card.Content>
	</Card.Root>

	<div class="grid grid-cols-1 md:grid-cols-2 gap-8">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Sparkles class="w-6 h-6 text-purple-600" />
					<Card.Title>Frequently Asked Questions</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="font-medium text-lg mb-1">Can I change plans later?</h3>
						<p class="text-gray-600">
							Yes, you can upgrade or downgrade your plan at any time. Changes take effect at the
							start of your next billing cycle.
						</p>
					</div>
					<div>
						<h3 class="font-medium text-lg mb-1">Is there a free trial for paid plans?</h3>
						<p class="text-gray-600">
							Yes, both Pro and Enterprise plans come with a 14-day free trial. No credit card
							required to start.
						</p>
					</div>
					<div>
						<h3 class="font-medium text-lg mb-1">What payment methods do you accept?</h3>
						<p class="text-gray-600">
							We accept all major credit cards, PayPal, and for Enterprise customers, we also offer
							invoicing.
						</p>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Need Help Choosing?</Card.Title>
				<Card.Description>Our team is here to help you find the right plan</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-6">
					Not sure which plan is right for you? Our team can help you assess your needs and
					recommend the best option for your use case.
				</p>
				<div class="space-y-4">
					<Button variant="outline" class="w-full">Schedule a Demo</Button>
					<Button variant="outline" class="w-full">Contact Sales</Button>
					<p class="text-sm text-gray-500 text-center mt-2">
						Or email us at <span class="text-purple-600">sales@eurora-labs.com</span>
					</p>
				</div>
			</Card.Content>
		</Card.Root>
	</div>
</div>
