<script lang="ts">
	import { Card, Button, Input } from '@eurora/ui';
	import { Mail, Phone, MapPin } from '@lucide/svelte';
</script>

<div class="container mx-auto px-4 py-16 max-w-5xl">
	<div class="text-center mb-12">
		<h1 class="text-4xl font-bold mb-4">Contact Us</h1>
		<p class="text-xl text-gray-600 max-w-2xl mx-auto">
			Have questions or want to learn more about Eurora? We're here to help. Reach out to our team.
		</p>
	</div>

	<div class="grid grid-cols-1 md:grid-cols-2 gap-8 mb-16">
		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Send Us a Message</Card.Title>
				<Card.Description>Fill out the form below and we'll get back to you soon.</Card.Description>
			</Card.Header>
			<Card.Content>
				<form class="space-y-4">
					<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
						<div class="space-y-2">
							<label for="name" class="text-sm font-medium">Name</label>
							<Input id="name" placeholder="Your name" />
						</div>
						<div class="space-y-2">
							<label for="email" class="text-sm font-medium">Email</label>
							<Input id="email" type="email" placeholder="your.email@example.com" />
						</div>
					</div>
					<div class="space-y-2">
						<label for="subject" class="text-sm font-medium">Subject</label>
						<Input id="subject" placeholder="How can we help you?" />
					</div>
					<div class="space-y-2">
						<label for="message" class="text-sm font-medium">Message</label>
						<textarea
							id="message"
							rows="5"
							class="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-purple-600"
							placeholder="Your message here..."
						></textarea>
					</div>
					<Button class="w-full">Send Message</Button>
				</form>
			</Card.Content>
		</Card.Root>

		<div class="space-y-6">
			<Card.Root class="p-6">
				<Card.Header>
					<Card.Title>Contact Information</Card.Title>
					<Card.Description>Reach out to us directly using the information below.</Card.Description>
				</Card.Header>
				<Card.Content>
					<div class="space-y-4">
						<div class="flex items-start space-x-3">
							<Mail class="w-5 h-5 text-purple-600 mt-1" />
							<div>
								<h3 class="font-medium">Email</h3>
								<p class="text-gray-600">contact@eurora-labs.com</p>
							</div>
						</div>
						<div class="flex items-start space-x-3">
							<Phone class="w-5 h-5 text-purple-600 mt-1" />
							<div>
								<h3 class="font-medium">Phone</h3>
								<p class="text-gray-600">+1 (555) 123-4567</p>
							</div>
						</div>
						<div class="flex items-start space-x-3">
							<MapPin class="w-5 h-5 text-purple-600 mt-1" />
							<div>
								<h3 class="font-medium">Office</h3>
								<p class="text-gray-600">
									123 Tech Plaza<br />
									San Francisco, CA 94105<br />
									United States
								</p>
							</div>
						</div>
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root class="p-6">
				<Card.Header>
					<Card.Title>Business Hours</Card.Title>
					<Card.Description>When you can reach our team</Card.Description>
				</Card.Header>
				<Card.Content>
					<div class="space-y-2">
						<div class="flex justify-between">
							<span>Monday - Friday</span>
							<span>9:00 AM - 6:00 PM</span>
						</div>
						<div class="flex justify-between">
							<span>Saturday</span>
							<span>10:00 AM - 4:00 PM</span>
						</div>
						<div class="flex justify-between">
							<span>Sunday</span>
							<span>Closed</span>
						</div>
					</div>
				</Card.Content>
			</Card.Root>
		</div>
	</div>

	<Card.Root class="p-6 mb-16">
		<Card.Header>
			<Card.Title>Frequently Asked Questions</Card.Title>
			<Card.Description>Quick answers to common questions</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="space-y-6">
				<div>
					<h3 class="font-medium text-lg mb-2">How do I get started with Eurora?</h3>
					<p class="text-gray-600">
						Getting started is easy! Simply download the app from our website or app store, create
						an account, and follow the onboarding process.
					</p>
				</div>
				<div>
					<h3 class="font-medium text-lg mb-2">Is Eurora available on all platforms?</h3>
					<p class="text-gray-600">
						Yes, Eurora is available on Windows, macOS, Linux, iOS, and Android devices.
					</p>
				</div>
				<div>
					<h3 class="font-medium text-lg mb-2">How secure is my data with Eurora?</h3>
					<p class="text-gray-600">
						We take security seriously. All data is encrypted end-to-end, and we never share your
						information with third parties without your explicit consent.
					</p>
				</div>
			</div>
		</Card.Content>
	</Card.Root>
</div>
