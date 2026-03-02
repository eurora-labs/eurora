<script lang="ts">
	import { Alert, AlertDescription } from '@eurora/ui/components/alert/index';
	import InfoIcon from '@lucide/svelte/icons/info';
</script>

<div>
	<h1 class="mb-4 text-4xl font-bold">Why Eurora</h1>
	<p class="mb-12 text-lg text-muted-foreground">
		There are dozens of AI assistants. Here's why we built another one — and why it might be the
		last one you need.
	</p>

	<div class="flex flex-col gap-12">
		<section>
			<h2 class="mb-2 text-2xl font-semibold">AI that knows what you're working on</h2>
			<p class="text-muted-foreground">
				Most AI tools start every conversation from scratch. You paste a snippet, explain the
				context, and hope the model understands. Eurora takes a different approach: it
				integrates directly with your operating system and browser to see what you see. When
				you ask a question, Eurora already knows which application is in focus, what page
				you're reading, and what files you're working with. No copy-paste, no context
				switching — just ask.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Your data stays yours</h2>
			<p class="text-muted-foreground">
				Every major AI assistant processes your data on servers you don't control, often
				governed by jurisdictions with weak privacy protections. Eurora stores all cloud data
				in sovereign European data centers under EU privacy law. But if that's not enough, you
				can run the entire stack locally — the backend, the database, and the AI model — on
				your own hardware. Nothing ever leaves your network.
			</p>
			<Alert class="mt-4">
				<InfoIcon />
				<AlertDescription>
					<p>
						Self-hosting uses Docker Compose and Ollama. See the
						<a
							href="/docs/self-hosting"
							class="text-primary underline underline-offset-4 hover:text-primary/80"
						>
							self-hosting guide</a
						> to get started in minutes.
					</p>
				</AlertDescription>
			</Alert>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Open source, not open-washing</h2>
			<p class="text-muted-foreground">
				Every line of Eurora's code is public — the desktop app, the backend, the browser
				extension, and the infrastructure that ties them together. This isn't a marketing
				claim with an asterisk. You can read the source, audit the encryption, verify that
				telemetry is opt-in, and build the entire application from scratch. The project is
				licensed under the Sustainable Use License with an eventual transition to Apache 2.0.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">One app, every platform</h2>
			<p class="text-muted-foreground">
				Eurora ships native desktop apps for macOS, Windows, and Linux — not an Electron
				wrapper, but a Tauri application backed by a Rust runtime. Browser extensions for
				Chrome, Firefox, Edge, and Safari feed page content into the same conversation. Your
				threads, your settings, and your context follow you across devices and platforms.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Built in Europe, funded by users</h2>
			<p class="text-muted-foreground">
				Eurora is a company based in The Netherlands, founded by two people who got tired of
				AI tools that treat user data as the product. There are no venture capital investors
				pushing for growth at the expense of privacy. The only revenue comes from
				subscriptions, which means the only incentive is to build something people genuinely
				want to use.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Flexible by design</h2>
			<p class="text-muted-foreground">
				Not everyone has the same requirements. Eurora supports multiple deployment models and
				AI providers so you can pick what works for you.
			</p>
			<div class="mt-4 grid grid-cols-1 gap-px overflow-hidden rounded-xl border bg-border md:grid-cols-2">
				<div class="bg-card p-6">
					<h3 class="mb-1 font-medium">Eurora Cloud</h3>
					<p class="text-sm text-muted-foreground">
						Sign up and start chatting. Data stays in EU data centers. Includes a free tier
						for local-only use.
					</p>
				</div>
				<div class="bg-card p-6">
					<h3 class="mb-1 font-medium">Self-hosted</h3>
					<p class="text-sm text-muted-foreground">
						Run the backend on your own machine with Docker Compose. Full control, zero
						external dependencies.
					</p>
				</div>
				<div class="bg-card p-6">
					<h3 class="mb-1 font-medium">Remote providers</h3>
					<p class="text-sm text-muted-foreground">
						Connect to OpenAI or any compatible API endpoint with your own API key.
					</p>
				</div>
				<div class="bg-card p-6">
					<h3 class="mb-1 font-medium">Ollama</h3>
					<p class="text-sm text-muted-foreground">
						Run open-weight models locally. Inference never leaves your machine.
					</p>
				</div>
			</div>
		</section>
	</div>
</div>
