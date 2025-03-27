
<script lang="ts">
	import { Card, Button, VideoCard, Sheet, buttonVariants, Skeleton, Input, Label, ScrollArea } from '@eurora/ui';
	import { ArrowRight, Brain, Shield, Zap, Globe, MessageSquare, KeyRound, Mic, Briefcase, GraduationCap, Mail, Linkedin, Github, Layers, Code, Sparkles } from 'lucide-svelte';
	import IntroModule from './intro_module.svelte';

    // import { SiGithub, SiLinkedin} from '@icons-pack/svelte-simple-icons';  

    import WaitlistForm from './waitlist_form.svelte';

    let inputValue = $state('');
    let purpleText = $state('');

    // Typing animation configuration
	const instantTyping = false;
	const firstPart = 'Explain ';
	const secondPart = 'this';
	const typingSpeed = instantTyping ? 0 : 150; // milliseconds per character
	const initialDelay = instantTyping ? 0 : 50; // milliseconds before typing starts
 
</script>

<div class="container mx-auto px-4 pb-16 max-w-5xl">
	<IntroModule />
    <!-- <div class="pt-16 col-span-4">s
        <div class="relative animate-grow">
            <div
                class="py-6 px-4 shadow-lg rounded-md border border-gray-300 bg-white w-full flex items-center"
                style="font-size: 54px; height: 131px;"
            >
                <div class="flex-grow">
                    <span class="text-black">{inputValue}</span>
                    <span class="text-purple-600">{purpleText}</span>
                </div>
                <Mic class="h-8 w-8 text-gray-400" />
            </div>
        </div>
    </div> -->
	<!-- Hero Section -->
	<div class="text-center mb-16">
		<h1 class="text-5xl font-bold mb-6">Intelligence Without Compromise</h1>
		<p class="text-xl text-gray-600 max-w-3xl mx-auto mb-8">
			Eurora is a fully Open Source AI assistant that understands context, respects your privacy, and works across
			all your devices. Experience AI on your own terms.
		</p>
		<!-- <div class="flex justify-center gap-4">
            <Sheet.Root>
                <Sheet.Trigger class={buttonVariants({ variant: "default" })}
                  >Join Waitlist</Sheet.Trigger
                >
                <Sheet.Content side="right">
                 
                  <ScrollArea class="h-screen">
                    <WaitlistForm portalId="242150186" formId="f0b52ee4-94ab-477c-9ac5-a13cb3086f9b" region="na2" />
                  </ScrollArea>
                  
                  <Sheet.Footer>
                    <Skeleton class="w-full h-screen" />
                    <Skeleton class="w-full h-screen" />
                    <Skeleton class="w-full h-screen" />
                   
                  </Sheet.Footer>
                </Sheet.Content>
              </Sheet.Root>
			
		</div> -->
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
				<!-- <Button variant="link" href="/features" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button> -->
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
				<!-- <Button variant="link" href="/open-source" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button> -->
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
				<!-- <Button variant="link" href="/privacy" class="p-0">
					Learn more
					<ArrowRight class="ml-1 h-4 w-4" />
				</Button> -->
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
	<!-- <div class="mb-16">
		<VideoCard.Card class="mx-auto aspect-[2/1] max-w-5xl mb-8">
			<VideoCard.Content
				mp4Src="https://www.youtube.com/embed/dQw4w9WgXcQ"
				class="aspect-[2/1]"
				alignment="left"
			>
				<VideoCard.Header>
					<VideoCard.Title>One Click To AI</VideoCard.Title>
					<VideoCard.Description>
                        Eurora uses a single interface to help with anything and everything you need.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div>

    <div class="mb-16">
		<VideoCard.Card class="mx-auto aspect-[2/1] max-w-5xl mb-8">
			<VideoCard.Content
				mp4Src="https://www.youtube.com/embed/dQw4w9WgXcQ"
				class="aspect-[2/1]"
				alignment="right"
			>
				<VideoCard.Header>
					<VideoCard.Title>Up to 98% faster responses</VideoCard.Title>
					<VideoCard.Description>
                        Eurora uses a single interface to help with anything and everything you need.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div>

    <div class="mb-16">
		<VideoCard.Card class="mx-auto aspect-[2/1] max-w-5xl mb-8">
			<VideoCard.Content
				mp4Src="https://www.youtube.com/embed/dQw4w9WgXcQ"
				class="aspect-[2/1]"
				alignment="left"
			>
				<VideoCard.Header>
					<VideoCard.Title>Use your own LLM's</VideoCard.Title>
					<VideoCard.Description>
                        Eurora uses a single interface to help with anything and everything you need.
					</VideoCard.Description>
				</VideoCard.Header>
			</VideoCard.Content>
		</VideoCard.Card>
	</div> -->

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

    <h2 class="text-3xl font-bold mb-8">Leadership Team</h2>

	<div class="grid grid-cols-1 md:grid-cols-3 gap-8 mb-16">
		<!-- CEO -->
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex flex-col items-center mb-4">
					<div class="w-32 h-32 rounded-full bg-gray-200 mb-4"></div>
					<Card.Title>Laura Thommen</Card.Title>
					<p class="text-purple-600 font-medium">Chief Executive Officer</p>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<Briefcase class="w-4 h-4 text-purple-600" /> Experience
						</h3>
						<p class="text-gray-600">
							15+ years in tech leadership, previously VP of Product at TechCorp and founder of
							PrivacyFirst.
						</p>
					</div>
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<GraduationCap class="w-4 h-4 text-purple-600" /> Education
						</h3>
						<p class="text-gray-600">MBA from Stanford, BS in Computer Science from MIT.</p>
					</div>
					<div class="flex justify-center gap-3 mt-4">
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Mail class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Linkedin class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Github class="h-4 w-4" />
						</Button>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- CTO -->
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex flex-col items-center mb-4">
					<div class="w-32 h-32 rounded-full bg-gray-200 mb-4"></div>
					<Card.Title>Andre Roelofs</Card.Title>
					<p class="text-purple-600 font-medium">Chief Technology Officer</p>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<Briefcase class="w-4 h-4 text-purple-600" /> Experience
						</h3>
						<p class="text-gray-600">
							Former AI Research Lead at OpenMind, contributed to multiple open-source ML
							frameworks.
						</p>
					</div>
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<GraduationCap class="w-4 h-4 text-purple-600" /> Education
						</h3>
						<p class="text-gray-600">
							PhD in Machine Learning from UC Berkeley, MS in Computer Science from Carnegie Mellon.
						</p>
					</div>
					<div class="flex justify-center gap-3 mt-4">
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Mail class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Linkedin class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Github class="h-4 w-4" />
						</Button>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- COO -->
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex flex-col items-center mb-4">
					<div class="w-32 h-32 rounded-full bg-gray-200 mb-4"></div>
					<Card.Title>Emily Rodriguez</Card.Title>
					<p class="text-purple-600 font-medium">Chief Operations Officer</p>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<Briefcase class="w-4 h-4 text-purple-600" /> Experience
						</h3>
						<p class="text-gray-600">
							10+ years in operations and scaling startups, previously COO at DataSecure and
							Director at TechScale.
						</p>
					</div>
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<GraduationCap class="w-4 h-4 text-purple-600" /> Education
						</h3>
						<p class="text-gray-600">
							MBA from Harvard Business School, BA in Business Administration from UCLA.
						</p>
					</div>
					<div class="flex justify-center gap-3 mt-4">
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Mail class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Linkedin class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Github class="h-4 w-4" />
						</Button>
					</div>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

    <h2 class="text-3xl font-bold mb-8">Features</h2>


    <div class="grid grid-cols-1 md:grid-cols-2 gap-8 mb-16">
  <Card.Root class="p-6">
   <Card.Header>
    <div class="flex items-center gap-2">
    	<Github class="w-6 h-6 text-purple-600" />
    	<Card.Title>Fully Open Source Hosting</Card.Title>
    </div>
    <Card.Description>Host Eurora on your own infrastructure</Card.Description>
   </Card.Header>
   <Card.Content>
    <p class="text-gray-600 mb-4">
    	Eurora is completely open source and can be self-hosted by individuals or companies.
    	Take full control of your AI assistant by running it on your own hardware, ensuring
    	complete data sovereignty and customization options.
    </p>
    <div class="bg-gray-50 p-4 rounded-md border border-gray-200">
    	<h4 class="font-medium mb-2">Hosting Benefits:</h4>
    	<ul class="list-disc pl-5 space-y-1 text-gray-600">
    		<li>Complete control over your data and infrastructure</li>
    		<li>Customizable deployment options for individuals and enterprises</li>
    		<li>No vendor lock-in or subscription fees</li>
    		<li>Community-supported deployment guides and documentation</li>
    	</ul>
    </div>
   </Card.Content>
  </Card.Root>

  <Card.Root class="p-6">
   <Card.Header>
    <div class="flex items-center gap-2">
    	<Code class="w-6 h-6 text-purple-600" />
    	<Card.Title>Built with Rust</Card.Title>
    </div>
    <Card.Description>Maximum security and safety by design</Card.Description>
   </Card.Header>
   <Card.Content>
    <p class="text-gray-600 mb-4">
    	Eurora is written in Rust, a language designed for performance, reliability, and security.
    	This implementation choice ensures memory safety, eliminates common vulnerabilities, and
    	provides robust protection for your sensitive data.
    </p>
    <div class="bg-gray-50 p-4 rounded-md border border-gray-200">
    	<h4 class="font-medium mb-2">Rust Advantages:</h4>
    	<ul class="list-disc pl-5 space-y-1 text-gray-600">
    		<li>Memory safety without garbage collection</li>
    		<li>Thread safety to prevent data races</li>
    		<li>Zero-cost abstractions for optimal performance</li>
    		<li>Comprehensive compile-time checks to catch errors early</li>
    	</ul>
    </div>
   </Card.Content>
  </Card.Root>

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
				<Card.Description>Seamless access to your files and productivity tools</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600 mb-6">
					Eurora seamlessly integrates with your file storage systems and productivity tools, giving you
					instant access to your content wherever it lives. Whether your files are stored in the cloud or
					on your device, Eurora can search, analyze, and help you work with them efficiently.
				</p>

				<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
					<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
						<h4 class="font-medium mb-2">File Storage</h4>
						<ul class="list-disc pl-5 space-y-1 text-gray-600">
							<li>Local device storage</li>
							<li>Google Drive</li>
							<li>Dropbox</li>
							<li>OneDrive</li>
							<li>iCloud</li>
						</ul>
					</div>

					<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
						<h4 class="font-medium mb-2">Productivity Apps</h4>
						<ul class="list-disc pl-5 space-y-1 text-gray-600">
							<li>Notion</li>
							<li>Obsidian</li>
							<li>Evernote</li>
							<li>OneNote</li>
							<li>Roam Research</li>
						</ul>
					</div>

					<div class="bg-gray-50 p-4 rounded-md border border-gray-200">
						<h4 class="font-medium mb-2">Workspace Tools</h4>
						<ul class="list-disc pl-5 space-y-1 text-gray-600">
							<li>Google Workspace</li>
							<li>Microsoft Office</li>
							<li>Slack</li>
							<li>Trello & Asana</li>
							<li>Airtable</li>
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
<style>
	/* Optional: Add a blinking cursor animation */
	@keyframes blink {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0;
		}
	}

	:global(.animate-blink) {
		animation: blink 1s infinite;
	}

	@keyframes grow {
		from {
			transform: scale(0.2);
		}
		to {
			transform: scale(1);
		}
	}

	.animate-grow {
		animation: grow var(--animation-duration) ease-in-out;
	}

	:global(.animate-grow) {
		--animation-duration: 300ms;
	}

	.card-entrance {
		opacity: 0;
		transform: translateY(20px);
		animation: slideIn 0.5s ease-out forwards;
		/* Add transition for smoother repositioning if needed */
		transition: all 0.3s ease-out;
	}

	@keyframes slideIn {
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}
</style>
