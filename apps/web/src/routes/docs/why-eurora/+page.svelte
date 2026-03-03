<script lang="ts">
	import { Alert, AlertDescription } from '@eurora/ui/components/alert/index';
	import InfoIcon from '@lucide/svelte/icons/info';
</script>

<div>
	<h1 class="mb-4 text-4xl font-bold">Why Eurora</h1>
	<p class="mb-12 text-lg text-muted-foreground">
		There are dozens of AI assistants. Here's why I built another one - and why it might be the
		last one you need.
	</p>

	<div class="flex flex-col gap-12">
		<section>
			<h2 class="mb-2 text-2xl font-semibold">Using AI is slow and painful</h2>
			<p class="text-muted-foreground">
				Most AI tools start every conversation from scratch. You paste a snippet, explain
				the context, and hope the model understands. Eurora takes a different approach: it
				integrates directly with your operating system and browser to see what you see. When
				you ask a question, Eurora already knows which application is in focus, what page
				you're reading, and what files you're working with. You don't need to adapt your
				workflows to fit the AI - Eurora adapts to you.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Local AI required compromise</h2>
			<p class="text-muted-foreground">
				Every single local-first AI assistant felt like a compromise. Like I had to use
				smaller, worse models just to get basic privacy. It felt like I had to use cloud
				based AI wrapped in proprietary code to actually get my work done.
			</p>
			<br />
			<p class="text-muted-foreground">
				There were some proprietary and open source apps that leveraged context to some
				degree. But all of them either ran fully on a single machine, or had a whole set of
				other issues. Most importantly, their context was limited and their integration with
				other apps - poor.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Asking "What is this"</h2>
			<p class="text-muted-foreground">
				Dissatisfied with current state of things, I built Eurora. It had a few
				requirements. The most important one of which was: ability to understand what I am
				doing right now. I wanted to ask "What do they mean?" and have AI have perfect
				understanding of every word. No more tiresome explaining. Every other requirement
				stemmed directly from that.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">
				Early realizations and commitment to privacy
			</h2>
			<p class="text-muted-foreground">
				As I kept developing Eurora more and more, I asked myself an important question:
				Would I actually ever use it myself? The answer was a resounding no. At that stage
				most of the code was proprietary. And I just knew I was not comfortable enough using
				it. In fact, even if the app was open source I would still never use it deployed on
				some data center like AWS or Azure.
			</p>
			<br />
			<p class="text-muted-foreground">
				Clearly I needed something different. The first decision I made was to create my
				company in The Netherlands. Two reasons made this decision attractive: Strong worker
				protection laws and European standards for privacy and data protection, which are
				the highest in the world.
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
