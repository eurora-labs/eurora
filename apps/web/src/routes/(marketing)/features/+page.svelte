<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import CodeIcon from '@lucide/svelte/icons/code';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import LayersIcon from '@lucide/svelte/icons/layers';
	import MessageSquareIcon from '@lucide/svelte/icons/message-square';
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import ZapIcon from '@lucide/svelte/icons/zap';
</script>

<div class="container mx-auto max-w-5xl px-4 py-16">
	<div class="mb-12 text-center">
		<h1 class="mb-4 text-4xl font-bold">Features</h1>
		<p class="mx-auto max-w-2xl text-xl text-gray-600">
			Discover what makes Eurora the most powerful AI assistant for your everyday tasks.
		</p>
	</div>

	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<BrainIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Advanced AI Understanding</Card.Title>
				</div>
				<Card.Description>Eurora understands context like never before</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora's advanced AI models can understand complex queries, remember context
					from previous threads, and provide more accurate and relevant responses than
					traditional AI assistants.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Key Capabilities:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Natural language understanding with contextual awareness</li>
						<li>Memory of previous interactions for more coherent threads</li>
						<li>Understanding of complex instructions with multiple steps</li>
						<li>Ability to learn from your preferences over time</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<ShieldIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Privacy-First Design</Card.Title>
				</div>
				<Card.Description>Your data stays private and secure</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Unlike other AI assistants, Eurora is designed with privacy at its core. Your
					data never leaves your device without your explicit permission, and you have
					complete control over what information is shared.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Privacy Features:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Local processing for sensitive information</li>
						<li>End-to-end encryption for all data transmissions</li>
						<li>Granular permission controls for data sharing</li>
						<li>Option to delete thread history at any time</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<ZapIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Lightning-Fast Performance</Card.Title>
				</div>
				<Card.Description>Get answers instantly, even offline</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora is optimized for speed, with local processing capabilities that allow it
					to function even without an internet connection. When online, it leverages cloud
					resources for more complex tasks while maintaining responsiveness.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Performance Highlights:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Instant responses for common queries</li>
						<li>Offline mode for essential functionality</li>
						<li>Optimized resource usage to preserve battery life</li>
						<li>Adaptive processing based on device capabilities</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<GlobeIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Cross-Platform Integration</Card.Title>
				</div>
				<Card.Description>Seamless experience across all your devices</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Use Eurora across all your devices with perfect synchronization. Whether you're
					on your phone, tablet, computer, or browser, Eurora provides a consistent
					experience with your preferences and history available everywhere.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Available Platforms:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Windows, macOS, and Linux desktop applications</li>
						<li>iOS and Android mobile apps</li>
						<li>Chrome, Firefox, and Edge browser extensions</li>
						<li>Secure cloud synchronization between devices</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<div class="mb-16">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<LayersIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Powerful Integrations</Card.Title>
				</div>
				<Card.Description>Connect with your favorite tools and services</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-6 text-gray-600">
					Eurora seamlessly integrates with the tools and services you already use, making
					it a central hub for your digital life. From productivity apps to creative
					tools, Eurora enhances your workflow without disrupting it.
				</p>

				<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
					<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
						<h4 class="mb-2 font-medium">Productivity</h4>
						<ul class="list-disc space-y-1 pl-5 text-gray-600">
							<li>Google Workspace</li>
							<li>Microsoft Office</li>
							<li>Notion</li>
							<li>Slack</li>
							<li>Trello & Asana</li>
						</ul>
					</div>

					<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
						<h4 class="mb-2 font-medium">Creative Tools</h4>
						<ul class="list-disc space-y-1 pl-5 text-gray-600">
							<li>Adobe Creative Cloud</li>
							<li>Figma</li>
							<li>Canva</li>
							<li>Midjourney</li>
							<li>DaVinci Resolve</li>
						</ul>
					</div>

					<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
						<h4 class="mb-2 font-medium">Development</h4>
						<ul class="list-disc space-y-1 pl-5 text-gray-600">
							<li>VS Code</li>
							<li>GitHub</li>
							<li>GitLab</li>
							<li>JetBrains IDEs</li>
							<li>Terminal environments</li>
						</ul>
					</div>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<MessageSquareIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Natural Threads</Card.Title>
				</div>
				<Card.Description>Talk to Eurora like you would to a human</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Eurora's threadal abilities go beyond simple command-response interactions. Have
					natural, flowing threads with follow-up questions, clarifications, and even
					humor.
				</p>
				<div class="mb-4 rounded-md border border-gray-200 bg-gray-50 p-4">
					<div class="mb-3 flex gap-2">
						<div
							class="flex h-8 w-8 items-center justify-center rounded-full bg-gray-300"
						>
							<span class="text-sm font-medium">You</span>
						</div>
						<div class="flex-1 rounded-md bg-blue-50 p-2">
							Can you help me plan a trip to Japan for next spring?
						</div>
					</div>
					<div class="mb-3 flex gap-2">
						<div
							class="flex h-8 w-8 items-center justify-center rounded-full bg-purple-300"
						>
							<span class="text-sm font-medium">AI</span>
						</div>
						<div class="flex-1 rounded-md bg-purple-50 p-2">
							I'd be happy to help plan your Japan trip! What are you most interested
							in experiencing there - culture, food, nature, or something else?
						</div>
					</div>
					<div class="flex gap-2">
						<div
							class="flex h-8 w-8 items-center justify-center rounded-full bg-gray-300"
						>
							<span class="text-sm font-medium">You</span>
						</div>
						<div class="flex-1 rounded-md bg-blue-50 p-2">
							I love food and traditional culture. And I'd like to see cherry
							blossoms.
						</div>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<CodeIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Developer-Friendly</Card.Title>
				</div>
				<Card.Description>Extend Eurora with custom plugins and APIs</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-4 text-gray-600">
					Developers can extend Eurora's capabilities with custom plugins and
					integrations. Our comprehensive API and SDK make it easy to build powerful
					extensions that leverage Eurora's AI capabilities.
				</p>
				<div class="rounded-md border border-gray-200 bg-gray-50 p-4">
					<h4 class="mb-2 font-medium">Developer Resources:</h4>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Open API with comprehensive documentation</li>
						<li>SDK for multiple programming languages</li>
						<li>Plugin marketplace for sharing and discovering extensions</li>
						<li>Developer community and support forums</li>
					</ul>
				</div>
				<div class="mt-4">
					<Button variant="outline" class="w-full">
						<CodeIcon class="mr-2 h-4 w-4" />
						Explore Developer Docs
					</Button>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<Card.Root class="p-6">
		<Card.Header>
			<div class="flex items-center gap-2">
				<SparklesIcon class="h-6 w-6 text-purple-600" />
				<Card.Title>Coming Soon</Card.Title>
			</div>
			<Card.Description>Exciting new features on our roadmap</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="grid grid-cols-1 gap-6 md:grid-cols-3">
				<div class="space-y-2">
					<h3 class="text-lg font-medium">Advanced Image Generation</h3>
					<p class="text-gray-600">
						Create stunning, customized images from text descriptions with our upcoming
						image generation feature.
					</p>
				</div>
				<div class="space-y-2">
					<h3 class="text-lg font-medium">Voice Assistant Mode</h3>
					<p class="text-gray-600">
						Interact with Eurora using just your voice, with advanced speech recognition
						and natural responses.
					</p>
				</div>
				<div class="space-y-2">
					<h3 class="text-lg font-medium">Collaborative Workspaces</h3>
					<p class="text-gray-600">
						Work together with teammates using shared AI workspaces for collaborative
						projects and brainstorming.
					</p>
				</div>
			</div>
		</Card.Content>
	</Card.Root>
</div>
