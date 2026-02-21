<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Input } from '@eurora/ui/components/input/index';
	import MailIcon from '@lucide/svelte/icons/mail';
	import MapPinIcon from '@lucide/svelte/icons/map-pin';
	import PhoneIcon from '@lucide/svelte/icons/phone';
</script>

<div class="container mx-auto max-w-5xl px-4 py-16">
	<div class="mb-12 text-center">
		<h1 class="mb-4 text-4xl font-bold">Contact Us</h1>
		<p class="mx-auto max-w-2xl text-xl text-muted-foreground">
			Have questions or want to learn more about Eurora? We're here to help. Reach out to our
			team.
		</p>
	</div>

	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Send Us a Message</Card.Title>
				<Card.Description
					>Fill out the form below and we'll get back to you soon.</Card.Description
				>
			</Card.Header>
			<Card.Content>
				<form class="space-y-4">
					<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
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
							class="w-full rounded-md border border-input px-3 py-2 focus:outline-none focus:ring-2 focus:ring-ring"
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
					<Card.Description
						>Reach out to us directly using the information below.</Card.Description
					>
				</Card.Header>
				<Card.Content>
					<div class="space-y-4">
						<div class="flex items-start space-x-3">
							<MailIcon class="mt-1 h-5 w-5 text-primary" />
							<div>
								<h3 class="font-medium">Email</h3>
								<p class="text-muted-foreground">contact@eurora-labs.com</p>
							</div>
						</div>
						<div class="flex items-start space-x-3">
							<PhoneIcon class="mt-1 h-5 w-5 text-primary" />
							<div>
								<h3 class="font-medium">Phone</h3>
								<p class="text-muted-foreground">+1 (555) 123-4567</p>
							</div>
						</div>
						<div class="flex items-start space-x-3">
							<MapPinIcon class="mt-1 h-5 w-5 text-primary" />
							<div>
								<h3 class="font-medium">Office</h3>
								<p class="text-muted-foreground">
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

	<Card.Root class="mb-16 p-6">
		<Card.Header>
			<Card.Title>Frequently Asked Questions</Card.Title>
			<Card.Description>Quick answers to common questions</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="space-y-6">
				<div>
					<h3 class="mb-2 text-lg font-medium">How do I get started with Eurora?</h3>
					<p class="text-muted-foreground">
						Getting started is easy! Simply download the app from our website or app
						store, create an account, and follow the onboarding process.
					</p>
				</div>
				<div>
					<h3 class="mb-2 text-lg font-medium">Is Eurora available on all platforms?</h3>
					<p class="text-muted-foreground">
						Yes, Eurora is available on Windows, macOS, Linux, iOS, and Android devices.
					</p>
				</div>
				<div>
					<h3 class="mb-2 text-lg font-medium">How secure is my data with Eurora?</h3>
					<p class="text-muted-foreground">
						We take security seriously. All data is encrypted end-to-end, and we never
						share your information with third parties without your explicit consent.
					</p>
				</div>
			</div>
		</Card.Content>
	</Card.Root>
</div>
