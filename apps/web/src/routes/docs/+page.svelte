<script lang="ts">
	import * as Card from '@eurora/ui/components/card/index';
	import ServerIcon from '@lucide/svelte/icons/server';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import MonitorIcon from '@lucide/svelte/icons/monitor';
	import ShieldCheckIcon from '@lucide/svelte/icons/shield-check';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import BrainCircuitIcon from '@lucide/svelte/icons/brain-circuit';
	import LayersIcon from '@lucide/svelte/icons/layers';
	import CodeIcon from '@lucide/svelte/icons/code';
	import PuzzleIcon from '@lucide/svelte/icons/puzzle';
</script>

<div>
	<h1 class="mb-4 text-4xl font-bold">Welcome to Eurora</h1>
	<p class="mb-12 text-lg text-muted-foreground">
		Eurora is a context-aware, cross-platform, open-source AI platform built in Europe. It
		integrates with your operating system and browser to give AI full awareness of what you're
		working on — without sending your data anywhere you don't control.
	</p>

	<div class="flex flex-col gap-12">
		<section>
			<h2 class="mb-4 text-2xl font-semibold">Key principles</h2>
			<div class="grid grid-cols-1 gap-px overflow-hidden rounded-xl border bg-border md:grid-cols-2">
				<div class="flex gap-3 bg-card p-6">
					<ShieldCheckIcon size={20} class="mt-0.5 shrink-0 text-primary" />
					<div>
						<h3 class="mb-1 font-medium">Privacy-first</h3>
						<p class="text-sm text-muted-foreground">
							All cloud data is stored in sovereign European infrastructure. Run fully
							locally with Ollama for complete data ownership.
						</p>
					</div>
				</div>
				<div class="flex gap-3 bg-card p-6">
					<BrainCircuitIcon size={20} class="mt-0.5 shrink-0 text-primary" />
					<div>
						<h3 class="mb-1 font-medium">Context-aware</h3>
						<p class="text-sm text-muted-foreground">
							Eurora sees what you see. It tracks your active application and browser
							content so you never have to copy-paste context into a chat window.
						</p>
					</div>
				</div>
				<div class="flex gap-3 bg-card p-6">
					<GlobeIcon size={20} class="mt-0.5 shrink-0 text-primary" />
					<div>
						<h3 class="mb-1 font-medium">Cross-platform</h3>
						<p class="text-sm text-muted-foreground">
							Native desktop apps for macOS, Windows, and Linux. Browser extensions for
							Chrome, Firefox, Edge, and Safari.
						</p>
					</div>
				</div>
				<div class="flex gap-3 bg-card p-6">
					<CodeIcon size={20} class="mt-0.5 shrink-0 text-primary" />
					<div>
						<h3 class="mb-1 font-medium">Open source</h3>
						<p class="text-sm text-muted-foreground">
							Every line of code is public. Audit it, fork it, or self-host it. Licensed
							under the Sustainable Use License with eventual Apache 2.0 transition.
						</p>
					</div>
				</div>
			</div>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Architecture</h2>
			<p class="mb-4 text-muted-foreground">
				Eurora is composed of three main components that work together to provide seamless AI
				assistance across your workflow.
			</p>
			<div class="flex flex-col gap-4">
				<div class="flex gap-3 rounded-xl border bg-card p-6">
					<MonitorIcon size={20} class="mt-0.5 shrink-0 text-primary" />
					<div>
						<h3 class="mb-1 font-medium">Desktop app</h3>
						<p class="text-sm text-muted-foreground">
							Built with Tauri v2 and Svelte. The desktop app is the main interface for
							chatting with AI. It uses OS-level accessibility APIs to track your active
							application, manages conversation threads, and handles authentication,
							settings, encryption, and auto-updates. The Rust backend communicates with
							the frontend via type-safe RPC bindings.
						</p>
					</div>
				</div>
				<div class="flex gap-3 rounded-xl border bg-card p-6">
					<LayersIcon size={20} class="mt-0.5 shrink-0 text-primary" />
					<div>
						<h3 class="mb-1 font-medium">Backend</h3>
						<p class="text-sm text-muted-foreground">
							A Rust monolith exposing both gRPC and HTTP services. Handles user
							authentication (password, OAuth via Google/GitHub), conversation storage,
							asset management with MIME validation, and AI inference routing to Ollama or
							remote providers. Uses PostgreSQL for persistence, OpenDAL for storage
							(filesystem or S3), and Casbin for authorization. Supports both cloud
							deployment and fully-local self-hosting via Docker Compose.
						</p>
					</div>
				</div>
				<div class="flex gap-3 rounded-xl border bg-card p-6">
					<PuzzleIcon size={20} class="mt-0.5 shrink-0 text-primary" />
					<div>
						<h3 class="mb-1 font-medium">Browser extension</h3>
						<p class="text-sm text-muted-foreground">
							Connects to the desktop app via native messaging to capture page content,
							extract assets from sites like YouTube and Twitter, and provide browser-level
							context to the AI. Available for Chrome, Firefox, Edge, and Safari.
						</p>
					</div>
				</div>
			</div>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Deployment options</h2>
			<p class="mb-4 text-muted-foreground">
				Eurora supports two deployment modes depending on your needs.
			</p>
			<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
				<div class="rounded-xl border bg-card p-6">
					<h3 class="mb-1 font-medium">Eurora Cloud</h3>
					<p class="text-sm text-muted-foreground">
						Sign up and start using Eurora immediately. Data is stored in sovereign European
						data centers. Includes a free tier for local-only use and a Pro tier for cloud
						features.
					</p>
				</div>
				<div class="rounded-xl border bg-card p-6">
					<h3 class="mb-1 font-medium">Self-hosted</h3>
					<p class="text-sm text-muted-foreground">
						Run the entire backend on your own machine with Docker Compose and Ollama. No
						data ever leaves your network. See the
						<a
							href="/docs/self-hosting"
							class="text-primary underline underline-offset-4 hover:text-primary/80"
						>
							self-hosting guide</a
						> for setup instructions.
					</p>
				</div>
			</div>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Guides</h2>
			<div class="grid gap-4">
				<Card.Root class="group">
					<a href="/docs/why-eurora" class="block p-6">
						<Card.Header class="p-0 pb-2">
							<div class="flex items-center gap-2">
								<SparklesIcon size={18} class="text-primary" />
								<Card.Title>Why Eurora</Card.Title>
							</div>
						</Card.Header>
						<Card.Content class="p-0">
							<p class="text-sm text-muted-foreground">
								What makes Eurora different — context-awareness, privacy, open source, and
								flexible deployment.
							</p>
						</Card.Content>
					</a>
				</Card.Root>
				<Card.Root class="group">
					<a href="/docs/self-hosting" class="block p-6">
						<Card.Header class="p-0 pb-2">
							<div class="flex items-center gap-2">
								<ServerIcon size={18} class="text-primary" />
								<Card.Title>Self-Hosting</Card.Title>
							</div>
						</Card.Header>
						<Card.Content class="p-0">
							<p class="text-sm text-muted-foreground">
								Run the Eurora backend on your own machine using Docker Compose with full
								control over your data and local models.
							</p>
						</Card.Content>
					</a>
				</Card.Root>
			</div>
		</section>
	</div>
</div>
