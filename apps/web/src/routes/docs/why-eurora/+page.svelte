<script lang="ts">
	import { Alert, AlertDescription } from '@eurora/ui/components/alert/index';
	import InfoIcon from '@lucide/svelte/icons/info';
</script>

<div>
	<h1 class="mb-4 text-4xl font-bold">Why Eurora</h1>
	<p class="mb-12 text-lg text-muted-foreground">
		There are dozens of AI clients already. Here's why I built another one.
	</p>

	<div class="flex flex-col gap-12">
		<section>
			<h2 class="mb-2 text-2xl font-semibold">The problem</h2>
			<p class="text-muted-foreground">
				Most AI tools start every conversation from scratch. You paste a snippet, explain
				the context, and hope the model understands. If you want privacy, the tradeoff is
				even worse. You're either stuck with smaller, weaker models running locally, or you
				hand everything to a cloud service wrapped in proprietary code.
			</p>
			<br />
			<p class="text-muted-foreground">
				There were some apps that tried to bridge this gap, both proprietary and open
				source. But all of them either ran fully on a single machine with limited context,
				or had poor integration with the tools you actually use day to day.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">What I actually wanted</h2>
			<p class="text-muted-foreground">
				I wanted to ask "What do they mean?" and have the AI understand every word. Which
				application is in focus, what page I'm reading, what files I'm working with. No more
				tiresome copy-pasting and explaining.
			</p>
			<br />
			<p class="text-muted-foreground">
				That meant building for every operating system and every browser people actually
				use. You don't adapt your workflow to Eurora. Eurora adapts to you.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Privacy and trust</h2>
			<p class="text-muted-foreground">
				As development progressed, I asked myself: would I actually use this? The honest
				answer was no. An app that sees everything on your screen has to earn a level of
				trust that proprietary software never can. So the code is open source, and the
				company is based in The Netherlands, subject to European privacy and data protection
				standards, the strictest in the world.
			</p>
			<br />
			<p class="text-muted-foreground">
				Because Eurora works the same way whether it's running in the cloud or on your own
				machine, you can always migrate your data from Eurora Cloud to a self-hosted
				instance. No lock-in, no partial exports. Everything moves with you.
			</p>
			<br />
			<p class="text-muted-foreground">
				There are no venture capital investors pushing for growth at the expense of privacy.
				The only revenue comes from subscriptions, which means the only incentive is to
				build something people genuinely want to use.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Flexible by design</h2>
			<p class="text-muted-foreground">
				Not everyone has the same requirements. Eurora supports multiple deployment models
				and AI providers so you can pick what works for you.
			</p>
			<div
				class="mt-4 grid grid-cols-1 gap-px overflow-hidden rounded-xl border bg-border md:grid-cols-2"
			>
				<div class="bg-card p-6">
					<h3 class="mb-1 font-medium">Eurora Cloud</h3>
					<p class="text-sm text-muted-foreground">
						Sign up and start chatting. Data stays in EU data centers. Includes a free
						tier for local-only use.
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
	</div>
</div>
