
<script lang="ts">
	import { Card, Button, VideoCard, Sheet, buttonVariants, Skeleton, Input, Label } from '@eurora/ui';
	import { ArrowRight, Brain, Shield, Zap, Globe, MessageSquare, KeyRound } from 'lucide-svelte';
	import IntroModule from './intro_module.svelte';

    import WaitlistForm from './waitlist_form.svelte';

 
</script>

<div class="container mx-auto px-4 py-16 max-w-5xl">
	<!-- <IntroModule /> -->
	<!-- Hero Section -->
	<div class="text-center mb-16">
		<h1 class="text-5xl font-bold mb-6">Intelligence Without Compromise</h1>
		<p class="text-xl text-gray-600 max-w-3xl mx-auto mb-8">
			Eurora is a fully Open Source AI assistant that understands context, respects your privacy, and works across
			all your devices. Experience AI on your own terms.
		</p>
		<div class="flex justify-center gap-4">
            <Sheet.Root>
                <Sheet.Trigger class={buttonVariants({ variant: "default" })}
                  >Sign Up</Sheet.Trigger
                >
                <Sheet.Content side="right">
                  <!-- <Sheet.Header>
                    <Sheet.Title>Edit profile</Sheet.Title>
                    <Sheet.Description>
                      Make changes to your profile here. Click save when you're done.
                    </Sheet.Description>
                  </Sheet.Header> -->
                  <WaitlistForm portalId="242150186" formId="f0b52ee4-94ab-477c-9ac5-a13cb3086f9b" region="na2" />
                  <!-- <div class="hs-form-frame" data-region="na2" data-form-id="f0b52ee4-94ab-477c-9ac5-a13cb3086f9b" data-portal-id="242150186"></div> -->
                  <!-- <div class="grid gap-4 py-4">
                    <div class="grid grid-cols-4 items-center gap-4">
                      <Label for="name" class="text-right">Name</Label>
                      <Input id="name" value="Pedro Duarte" class="col-span-3" />
                    </div>
                    <div class="grid grid-cols-4 items-center gap-4">
                      <Label for="username" class="text-right">Username</Label>
                      <Input id="username" value="@peduarte" class="col-span-3" />
                    </div>
                  </div> -->
                  <Sheet.Footer>
                    <Skeleton class="h-10 w-full" />
                    <!-- <Sheet.Close class={buttonVariants({ variant: "outline" })}
                      >Save changes</Sheet.Close
                    > -->
                  </Sheet.Footer>
                </Sheet.Content>
              </Sheet.Root>
			<!-- <Button href="/download" size="lg" class="px-6">
				Get Started
				<ArrowRight class="ml-2 h-5 w-5" />
			</Button> -->
			<Button href="/features" variant="outline" size="lg" class="px-6">Learn More</Button>
		</div>
	</div>

	<!-- Feature Highlights -->
	<div class="grid grid-cols-1 md:grid-cols-3 gap-8 mb-16">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Brain class="w-6 h-6 text-purple-600" />
					<Card.Title>Intelligent Understanding</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Eurora understands context and remembers previous conversations, providing more accurate
					and relevant responses than traditional AI assistants.
				</p>
				<Button variant="link" href="/features" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button>
			</Card.Content>
		</Card.Root>
        <Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<KeyRound class="w-6 h-6 text-purple-600" />
					<Card.Title>Fully Open Source</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Eurora is gives you full access to the code that runs on your device and handles your data. You can even run both the app and server on your own hardware as well as connect LLM's of your choosing.
				</p>
				<Button variant="link" href="/open-source" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Shield class="w-6 h-6 text-purple-600" />
					<Card.Title>Privacy-First Design</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Your data never leaves your device without your permission. Eurora is designed with
					privacy at its core, giving you complete control over your information.
				</p>
				<Button variant="link" href="/privacy" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button>
			</Card.Content>
		</Card.Root>

		<!-- <Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Globe class="w-6 h-6 text-purple-600" />
					<Card.Title>Works Everywhere</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Use Eurora across all your devices with perfect synchronization. Available on Windows,
					macOS, Linux, iOS, Android, and as browser extensions.
				</p>
				<Button variant="link" href="/download" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button>
			</Card.Content>
		</Card.Root> -->
	</div>

	<!-- Video Showcase -->
	<div class="mb-16">
		<VideoCard.Card class="mx-auto aspect-[2/1] max-w-5xl mb-8">
			<VideoCard.Content
				mp4Src="https://www.youtube.com/embed/dQw4w9WgXcQ"
				class="aspect-[2/1]"
				alignment="left"
			>
				<VideoCard.Header>
					<VideoCard.Title>One Click To Infinity</VideoCard.Title>
					<VideoCard.Description>
						<!-- Eurora can analyze and understand video content, allowing you to ask questions about
						what you're watching in real-time. -->
                        Eurora uses a single interface to help with anything and everything you need.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div>

	<!-- Use Cases -->
	<div class="mb-16">
		<h2 class="text-3xl font-bold text-center mb-8">How People Use Eurora</h2>
		<div class="grid grid-cols-1 md:grid-cols-2 gap-8">
            <Card.Root class="p-6">
				<Card.Header>
					<Card.Title>For Learning</Card.Title>
					<Card.Description>Enhance your education and skill development</Card.Description>
				</Card.Header>
				<Card.Content>
					<ul class="space-y-3 mb-4">
						<li class="flex items-start">
							<div
								class="w-5 h-5 rounded-full bg-purple-100 flex items-center justify-center mr-2 mt-0.5"
							>
								<span class="text-purple-600 text-xs">✓</span>
							</div>
							<span>Get instant explanations on complex topics</span>
						</li>
						<li class="flex items-start">
							<div
								class="w-5 h-5 rounded-full bg-purple-100 flex items-center justify-center mr-2 mt-0.5"
							>
								<span class="text-purple-600 text-xs">✓</span>
							</div>
							<span>Get real-time translation of live lectures</span>
						</li>
						<li class="flex items-start">
							<div
								class="w-5 h-5 rounded-full bg-purple-100 flex items-center justify-center mr-2 mt-0.5"
							>
								<span class="text-purple-600 text-xs">✓</span>
							</div>
							<span>Visualize homework problems and assignments</span>
						</li>
						<li class="flex items-start">
							<div
								class="w-5 h-5 rounded-full bg-purple-100 flex items-center justify-center mr-2 mt-0.5"
							>
								<span class="text-purple-600 text-xs">✓</span>
							</div>
							<span>Ask how new knowledge relates to previous concepts you've learned </span>
						</li>
					</ul>
				</Card.Content>
			</Card.Root>
			<Card.Root class="p-6">
				<Card.Header>
					<Card.Title>For Work</Card.Title>
					<Card.Description>Boost your productivity and streamline workflows</Card.Description>
				</Card.Header>
				<Card.Content>
					<ul class="space-y-3 mb-4">
						<li class="flex items-start">
							<div
								class="w-5 h-5 rounded-full bg-purple-100 flex items-center justify-center mr-2 mt-0.5"
							>
								<span class="text-purple-600 text-xs">✓</span>
							</div>
							<span>Ask question about any document you're reading</span>
						</li>
						<li class="flex items-start">
							<div
								class="w-5 h-5 rounded-full bg-purple-100 flex items-center justify-center mr-2 mt-0.5"
							>
								<span class="text-purple-600 text-xs">✓</span>
							</div>
							<span>Summarize long documents and research papers</span>
						</li>
						<li class="flex items-start">
							<div
								class="w-5 h-5 rounded-full bg-purple-100 flex items-center justify-center mr-2 mt-0.5"
							>
								<span class="text-purple-600 text-xs">✓</span>
							</div>
							<span>Get short description of the work you did yesterday</span>
						</li>
						<li class="flex items-start">
							<div
								class="w-5 h-5 rounded-full bg-purple-100 flex items-center justify-center mr-2 mt-0.5"
							>
								<span class="text-purple-600 text-xs">✓</span>
							</div>
							<span>Integrate with your existing productivity tools</span>
						</li>
					</ul>
				</Card.Content>
			</Card.Root>

			
		</div>
	</div>

	<!-- Testimonials -->
	<div class="mb-16">
		<h2 class="text-3xl font-bold text-center mb-8">
			What Our Users Say About Intelligence Without Compromise
		</h2>
		<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
			<Card.Root class="p-6">
				<Card.Content>
					<p class="text-gray-600 italic mb-4">
						"Eurora has completely transformed how I work. The privacy-first approach means I can
						use AI without worrying about my sensitive data."
					</p>
					<div class="flex items-center">
						<div class="w-10 h-10 rounded-full bg-gray-200 mr-3"></div>
						<div>
							<p class="font-medium">Sarah Johnson</p>
							<p class="text-sm text-gray-500">Product Manager</p>
						</div>
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root class="p-6">
				<Card.Content>
					<p class="text-gray-600 italic mb-4">
						"As a developer, I appreciate how Eurora integrates with all my tools. The API is
						well-documented and the customization options are fantastic."
					</p>
					<div class="flex items-center">
						<div class="w-10 h-10 rounded-full bg-gray-200 mr-3"></div>
						<div>
							<p class="font-medium">Michael Chen</p>
							<p class="text-sm text-gray-500">Software Engineer</p>
						</div>
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root class="p-6">
				<Card.Content>
					<p class="text-gray-600 italic mb-4">
						"I use Eurora daily for my studies. It helps me understand complex topics and saves me
						hours of research time. The contextual understanding is impressive."
					</p>
					<div class="flex items-center">
						<div class="w-10 h-10 rounded-full bg-gray-200 mr-3"></div>
						<div>
							<p class="font-medium">Emily Rodriguez</p>
							<p class="text-sm text-gray-500">Graduate Student</p>
						</div>
					</div>
				</Card.Content>
			</Card.Root>
		</div>
	</div>

	<!-- Call to Action -->
	<Card.Root class="p-8 bg-purple-50 border-none">
		<Card.Content class="text-center">
			<h2 class="text-3xl font-bold mb-4">Ready to Experience Intelligence Without Compromise?</h2>
			<p class="text-xl text-gray-600 max-w-2xl mx-auto mb-6">
				Join thousands of users who are already experiencing the future of AI assistance.
			</p>
			<div class="flex justify-center gap-4">
				<Button href="/download" size="lg" class="px-8">Download Now</Button>
				<Button href="/pricing" variant="outline" size="lg" class="px-8">View Pricing</Button>
			</div>
		</Card.Content>
	</Card.Root>
</div>
