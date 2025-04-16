<script lang="ts">
	import { Card, Button } from '@eurora/ui';
	import { Sparkles, Brain, Shield, Zap, Globe, Layers, MessageSquare, Code } from '@lucide/svelte';
</script>

<div class="container mx-auto px-4 py-16 max-w-5xl">
	<div class="text-center mb-12">
		<h1 class="text-4xl font-bold mb-4">Features</h1>
		<p class="text-xl text-gray-600 max-w-2xl mx-auto">
			Discover what makes Eurora the most powerful AI assistant for your everyday tasks.
		</p>
	</div>

	<div class="grid grid-cols-1 md:grid-cols-2 gap-8 mb-16">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Brain class="w-6 h-6 text-purple-600" />
					<Card.Title>Advanced AI Understanding</Card.Title>
				</div>
				<Card.Description>Eurora understands context like never before</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Eurora's advanced AI models can understand complex queries, remember context from previous
					conversations, and provide more accurate and relevant responses than traditional AI
					assistants.
				</p>
				<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
					<h4 class="font-medium mb-2">Key Capabilities:</h4>
					<ul class="list-disc pl-5 space-y-1 text-gray-600">
						<li>Natural language understanding with contextual awareness</li>
						<li>Memory of previous interactions for more coherent conversations</li>
						<li>Understanding of complex instructions with multiple steps</li>
						<li>Ability to learn from your preferences over time</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Shield class="w-6 h-6 text-purple-600" />
					<Card.Title>Privacy-First Design</Card.Title>
				</div>
				<Card.Description>Your data stays private and secure</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Unlike other AI assistants, Eurora is designed with privacy at its core. Your data never
					leaves your device without your explicit permission, and you have complete control over
					what information is shared.
				</p>
				<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
					<h4 class="font-medium mb-2">Privacy Features:</h4>
					<ul class="list-disc pl-5 space-y-1 text-gray-600">
						<li>Local processing for sensitive information</li>
						<li>End-to-end encryption for all data transmissions</li>
						<li>Granular permission controls for data sharing</li>
						<li>Option to delete conversation history at any time</li>
					</ul>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Zap class="w-6 h-6 text-purple-600" />
					<Card.Title>Lightning-Fast Performance</Card.Title>
				</div>
				<Card.Description>Get answers instantly, even offline</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Eurora is optimized for speed, with local processing capabilities that allow it to
					function even without an internet connection. When online, it leverages cloud resources
					for more complex tasks while maintaining responsiveness.
				</p>
				<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
					<h4 class="font-medium mb-2">Performance Highlights:</h4>
					<ul class="list-disc pl-5 space-y-1 text-gray-600">
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
					<Globe class="w-6 h-6 text-purple-600" />
					<Card.Title>Cross-Platform Integration</Card.Title>
				</div>
				<Card.Description>Seamless experience across all your devices</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Use Eurora across all your devices with perfect synchronization. Whether you're on your
					phone, tablet, computer, or browser, Eurora provides a consistent experience with your
					preferences and history available everywhere.
				</p>
				<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
					<h4 class="font-medium mb-2">Available Platforms:</h4>
					<ul class="list-disc pl-5 space-y-1 text-gray-600">
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
					<Layers class="w-6 h-6 text-purple-600" />
					<Card.Title>Powerful Integrations</Card.Title>
				</div>
				<Card.Description>Connect with your favorite tools and services</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-6">
					Eurora seamlessly integrates with the tools and services you already use, making it a
					central hub for your digital life. From productivity apps to creative tools, Eurora
					enhances your workflow without disrupting it.
				</p>

				<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
					<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
						<h4 class="font-medium mb-2">Productivity</h4>
						<ul class="list-disc pl-5 space-y-1 text-gray-600">
							<li>Google Workspace</li>
							<li>Microsoft Office</li>
							<li>Notion</li>
							<li>Slack</li>
							<li>Trello & Asana</li>
						</ul>
					</div>

					<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
						<h4 class="font-medium mb-2">Creative Tools</h4>
						<ul class="list-disc pl-5 space-y-1 text-gray-600">
							<li>Adobe Creative Cloud</li>
							<li>Figma</li>
							<li>Canva</li>
							<li>Midjourney</li>
							<li>DaVinci Resolve</li>
						</ul>
					</div>

					<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
						<h4 class="font-medium mb-2">Development</h4>
						<ul class="list-disc pl-5 space-y-1 text-gray-600">
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

	<div class="grid grid-cols-1 md:grid-cols-2 gap-8 mb-16">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<MessageSquare class="w-6 h-6 text-purple-600" />
					<Card.Title>Natural Conversations</Card.Title>
				</div>
				<Card.Description>Talk to Eurora like you would to a human</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Eurora's conversational abilities go beyond simple command-response interactions. Have
					natural, flowing conversations with follow-up questions, clarifications, and even humor.
				</p>
				<div class="bg-gray-50 p-4 rounded-md border border-gray-200 mb-4">
					<div class="flex gap-2 mb-3">
						<div class="w-8 h-8 rounded-full bg-gray-300 flex items-center justify-center">
							<span class="text-sm font-medium">You</span>
						</div>
						<div class="flex-1 bg-blue-50 p-2 rounded-md">
							Can you help me plan a trip to Japan for next spring?
						</div>
					</div>
					<div class="flex gap-2 mb-3">
						<div class="w-8 h-8 rounded-full bg-purple-300 flex items-center justify-center">
							<span class="text-sm font-medium">AI</span>
						</div>
						<div class="flex-1 bg-purple-50 p-2 rounded-md">
							I'd be happy to help plan your Japan trip! What are you most interested in
							experiencing there - culture, food, nature, or something else?
						</div>
					</div>
					<div class="flex gap-2">
						<div class="w-8 h-8 rounded-full bg-gray-300 flex items-center justify-center">
							<span class="text-sm font-medium">You</span>
						</div>
						<div class="flex-1 bg-blue-50 p-2 rounded-md">
							I love food and traditional culture. And I'd like to see cherry blossoms.
						</div>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<Code class="w-6 h-6 text-purple-600" />
					<Card.Title>Developer-Friendly</Card.Title>
				</div>
				<Card.Description>Extend Eurora with custom plugins and APIs</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-4">
					Developers can extend Eurora's capabilities with custom plugins and integrations. Our
					comprehensive API and SDK make it easy to build powerful extensions that leverage Eurora's
					AI capabilities.
				</p>
				<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
					<h4 class="font-medium mb-2">Developer Resources:</h4>
					<ul class="list-disc pl-5 space-y-1 text-gray-600">
						<li>Open API with comprehensive documentation</li>
						<li>SDK for multiple programming languages</li>
						<li>Plugin marketplace for sharing and discovering extensions</li>
						<li>Developer community and support forums</li>
					</ul>
				</div>
				<div class="mt-4">
					<Button variant="outline" class="w-full">
						<Code class="w-4 h-4 mr-2" />
						Explore Developer Docs
					</Button>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<Card.Root class="p-6">
		<Card.Header>
			<div class="flex items-center gap-2">
				<Sparkles class="w-6 h-6 text-purple-600" />
				<Card.Title>Coming Soon</Card.Title>
			</div>
			<Card.Description>Exciting new features on our roadmap</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="grid grid-cols-1 md:grid-cols-3 gap-6">
				<div class="space-y-2">
					<h3 class="font-medium text-lg">Advanced Image Generation</h3>
					<p class="text-gray-600">
						Create stunning, customized images from text descriptions with our upcoming image
						generation feature.
					</p>
				</div>
				<div class="space-y-2">
					<h3 class="font-medium text-lg">Voice Assistant Mode</h3>
					<p class="text-gray-600">
						Interact with Eurora using just your voice, with advanced speech recognition and natural
						responses.
					</p>
				</div>
				<div class="space-y-2">
					<h3 class="font-medium text-lg">Collaborative Workspaces</h3>
					<p class="text-gray-600">
						Work together with teammates using shared AI workspaces for collaborative projects and
						brainstorming.
					</p>
				</div>
			</div>
		</Card.Content>
	</Card.Root>
</div>
