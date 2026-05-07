<script lang="ts">
	import * as CodeBlock from '@eurora/ui/components/ai-elements/code-block/index';
	import { Alert, AlertDescription } from '@eurora/ui/components/alert/index';
	import InfoIcon from '@lucide/svelte/icons/info';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	const cloneCommand = 'git clone https://github.com/eurora-labs/eurora && cd eurora';
	const justDevCommand = 'just bootstrap && $EDITOR .env && just dev';

	const envFileContents = `# LLM provider — the only value 'just dev' needs you to set.
OPENAI_API_KEY=sk-...
EURORA_CHAT_MODEL=gpt-4o-mini

# Web frontend — Vite-exposed.
VITE_API_URL=http://localhost:3000

# Debug builds default REMOTE_DATABASE_URL to docker-compose's local
# Postgres, JWT secrets to stable placeholders, CORS origins to
# localhost+tauri://localhost, AUTH_COOKIE_SECURE to false, and asset
# storage to ./assets — uncomment any of those in .env to override.
# Release builds refuse to start without REMOTE_DATABASE_URL and the
# JWT secrets.`;

	const ollamaEnvContents = `# Replace the OPENAI_API_KEY block with these four lines.
EURORA_LLM_KIND=openai_compatible
EURORA_LLM_BASE_URL=http://localhost:11434/v1
EURORA_LLM_API_KEY=
EURORA_CHAT_MODEL=llama3.2`;

	const envVars = [
		{
			name: 'OPENAI_API_KEY',
			default: '—',
			description:
				'Required when EURORA_LLM_KIND is omitted or set to "openai". Drop this when pointing at an OpenAI-compatible server.',
			required: true,
		},
		{
			name: 'EURORA_CHAT_MODEL',
			default: '—',
			description: 'Model used for chat (and title, unless EURORA_TITLE_MODEL overrides).',
			required: true,
		},
		{
			name: 'EURORA_LLM_KIND',
			default: 'openai',
			description:
				'Either "openai" (default) or "openai_compatible". Other kinds are not yet wired.',
			required: false,
		},
		{
			name: 'EURORA_LLM_BASE_URL',
			default: '—',
			description: 'Required for "openai_compatible". Optional override for "openai".',
			required: false,
		},
		{
			name: 'EURORA_LLM_API_KEY',
			default: '—',
			description: 'API key for "openai_compatible" servers that require one.',
			required: false,
		},
		{
			name: 'EURORA_TITLE_MODEL',
			default: 'EURORA_CHAT_MODEL',
			description: 'Model used for thread title generation.',
			required: false,
		},
		{
			name: 'EURORA_VISION_MODEL',
			default: '—',
			description: 'When set, image-bearing messages are routed to this model.',
			required: false,
		},
		{
			name: 'VITE_API_URL',
			default: 'http://localhost:3000',
			description:
				'Backend URL the SvelteKit web app talks to. Vite exposes this to client code at build time.',
			required: true,
		},
		{
			name: 'JWT_ACCESS_SECRET / JWT_REFRESH_SECRET',
			default: 'dev placeholder (debug builds only)',
			description:
				'Random strings used to sign JWTs. Generate with `openssl rand -hex 32`. Required in release builds.',
			required: true,
		},
		{
			name: 'REMOTE_DATABASE_URL',
			default: 'docker-compose Postgres (debug builds only)',
			description:
				'PostgreSQL connection string. Required in release builds; debug builds fall back to the local docker-compose Postgres.',
			required: true,
		},
		{
			name: 'EURORA_API_BASE_URL',
			default: '—',
			description:
				'Desktop/mobile escape hatch: forces the app to talk to this URL on a single run, ignoring the persisted connection-mode setting.',
			required: false,
		},
		{
			name: 'EURORA_AUTH_SERVICE_URL',
			default: 'https://www.eurora-labs.com',
			description:
				'Where the OAuth/login page is served. Set to http://localhost:5173 in dev so the desktop app uses the local SvelteKit auth UI.',
			required: false,
		},
	];
</script>

<svelte:head>
	<title>Self-Hosting - Eurora Labs</title>
	<meta
		name="description"
		content="Self-host Eurora locally with one command. The desktop app talks to the backend over HTTP — point it at OpenAI, an OpenAI-compatible local server, or your own deployment."
	/>
</svelte:head>

<div>
	<h1 class="mb-4 text-4xl font-bold">Self-Hosting</h1>
	<p class="mb-12 text-lg text-muted-foreground">
		Run the Eurora backend on your own machine. The fastest path is the bundled
		<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">just dev</code>
		recipe — it spins up Postgres, seeds a development user, and runs the backend natively against
		an LLM provider you choose.
	</p>

	<div class="flex flex-col gap-12">
		<section>
			<h2 class="mb-2 text-2xl font-semibold">Prerequisites</h2>
			<ul class="list-disc space-y-2 pl-5 text-muted-foreground">
				<li>
					<strong>Docker &amp; Docker Compose</strong> — used to run Postgres and the seed
					container. Follow the
					<a
						href="https://docs.docker.com/get-docker/"
						target="_blank"
						rel="noopener noreferrer"
						class="text-primary underline underline-offset-4 hover:text-primary/80"
					>
						official installation guide</a
					>.
				</li>
				<li>
					<strong>Rust</strong> — the backend runs natively on your host for fast
					iteration. Install via
					<a
						href="https://rustup.rs"
						target="_blank"
						rel="noopener noreferrer"
						class="text-primary underline underline-offset-4 hover:text-primary/80"
						>rustup</a
					>.
				</li>
				<li>
					<strong>pnpm</strong> — used by the web and desktop frontends. The easiest path
					is
					<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
						>corepack enable</code
					>; alternatives are listed at
					<a
						href="https://pnpm.io/installation"
						target="_blank"
						rel="noopener noreferrer"
						class="text-primary underline underline-offset-4 hover:text-primary/80"
						>pnpm.io</a
					>.
				</li>
				<li>
					<strong>just</strong> — the task runner that orchestrates the local stack.
					Install with
					<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
						>cargo install just</code
					>.
				</li>
			</ul>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Quickstart</h2>
			<p class="mb-3 text-muted-foreground">Clone the repo and run the dev recipe:</p>
			<CodeBlock.Root code={cloneCommand} language="shellscript">
				<CodeBlock.Header>
					<CodeBlock.Actions class="ml-auto">
						<CodeBlock.CopyButton />
					</CodeBlock.Actions>
				</CodeBlock.Header>
			</CodeBlock.Root>
			<div class="mt-3">
				<CodeBlock.Root code={justDevCommand} language="shellscript">
					<CodeBlock.Header>
						<CodeBlock.Actions class="ml-auto">
							<CodeBlock.CopyButton />
						</CodeBlock.Actions>
					</CodeBlock.Header>
				</CodeBlock.Root>
			</div>
			<p class="mt-3 text-sm text-muted-foreground">
				That's it. <code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
					>just bootstrap</code
				>
				copies
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env.example</code>
				to
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code> and runs
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">pnpm install</code>;
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">just dev</code>
				brings up Postgres, seeds a
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">dev@dev.com</code>
				user (password
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">dev</code>), and runs
				the backend, the web auth UI, and the desktop app. The single
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code> at the repo
				root is the contract — every consumer (backend, Vite for web/desktop, mobile build) reads
				from there.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">.env reference</h2>
			<p class="mb-3 text-muted-foreground">
				The only value you need to fill in for the default flow is
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">OPENAI_API_KEY</code
				>:
			</p>
			<CodeBlock.Root code={envFileContents} language="bash">
				<CodeBlock.Header>
					<CodeBlock.Filename>.env</CodeBlock.Filename>
					<CodeBlock.Actions>
						<CodeBlock.CopyButton />
					</CodeBlock.Actions>
				</CodeBlock.Header>
			</CodeBlock.Root>
			<Alert variant="destructive" class="mt-3">
				<TriangleAlertIcon />
				<AlertDescription>
					<p>
						Debug builds fall back to placeholder JWT secrets so a fresh checkout runs
						without setup. Release builds refuse to start without explicit
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>JWT_ACCESS_SECRET</code
						>
						and
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>JWT_REFRESH_SECRET</code
						>
						values — for any deployment reachable from outside your machine, set them to long
						random strings (e.g.
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>openssl rand -hex 32</code
						>).
					</p>
				</AlertDescription>
			</Alert>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Pointing at a local model</h2>
			<p class="mb-3 text-muted-foreground">
				To run with a local OpenAI-compatible server like Ollama, LM Studio, or vLLM, swap
				the OpenAI block in your
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code> for:
			</p>
			<CodeBlock.Root code={ollamaEnvContents} language="bash">
				<CodeBlock.Header>
					<CodeBlock.Filename>.env (Ollama)</CodeBlock.Filename>
					<CodeBlock.Actions>
						<CodeBlock.CopyButton />
					</CodeBlock.Actions>
				</CodeBlock.Header>
			</CodeBlock.Root>
			<p class="mt-3 text-sm text-muted-foreground">
				Ollama serves an OpenAI-compatible API at
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
					>http://localhost:11434/v1</code
				>
				when you run
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">ollama serve</code>.
				The same env-var shape works for any other OpenAI-compatible endpoint.
			</p>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">Connect the desktop app</h2>
			<p class="mb-3 text-muted-foreground">
				The desktop app picks its backend in <strong>Settings &rarr; Connection</strong>:
			</p>
			<ul class="list-disc space-y-2 pl-5 text-muted-foreground">
				<li>
					<strong>Eurora Cloud</strong> — the hosted backend at api.eurora-labs.com.
				</li>
				<li>
					<strong>Local</strong> — what
					<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">just dev</code>
					brings up at
					<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
						>http://localhost:3000</code
					>.
				</li>
				<li>
					<strong>Custom</strong> — any URL you self-host at.
				</li>
			</ul>
			<Alert class="mt-3">
				<InfoIcon />
				<AlertDescription>
					<p>
						The "Test connection" button hits
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>/llm/info</code
						>
						on the chosen URL and surfaces the active model in a toast — useful for confirming
						you're talking to the right backend before saving.
					</p>
				</AlertDescription>
			</Alert>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Environment variable reference</h2>
			<p class="mb-3 text-muted-foreground">
				Set these in your <code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
					>.env</code
				> file.
			</p>
			<div class="overflow-x-auto rounded-lg border">
				<table class="w-full text-sm">
					<thead>
						<tr class="border-b bg-muted/50">
							<th class="px-4 py-2 text-left font-medium">Variable</th>
							<th class="px-4 py-2 text-left font-medium">Default</th>
							<th class="px-4 py-2 text-left font-medium">Description</th>
						</tr>
					</thead>
					<tbody>
						{#each envVars as v}
							<tr class="border-b last:border-0">
								<td class="px-4 py-2 font-mono text-xs">
									{v.name}
									{#if v.required}
										<span class="ml-1 text-destructive">*</span>
									{/if}
								</td>
								<td class="px-4 py-2 font-mono text-xs">{v.default}</td>
								<td class="px-4 py-2 text-muted-foreground">{v.description}</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
			<p class="mt-2 text-xs text-muted-foreground">
				<span class="text-destructive">*</span> Required for production deployments.
			</p>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Troubleshooting</h2>
			<div class="flex flex-col gap-4">
				<div>
					<h3 class="mb-1 font-medium">
						Backend exits with "LLM configuration is invalid"
					</h3>
					<p class="text-sm text-muted-foreground">
						The backend reads its provider config from environment variables at startup.
						The most common cause is forgetting
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>OPENAI_API_KEY</code
						>
						or
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>EURORA_CHAT_MODEL</code
						>. The error message names the missing variable.
					</p>
				</div>
				<div>
					<h3 class="mb-1 font-medium">Postgres port already in use</h3>
					<p class="text-sm text-muted-foreground">
						If port 5432 conflicts with another service, set
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>EURORA_POSTGRES_PORT</code
						>
						in your
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code> to
						a free port.
					</p>
				</div>
				<div>
					<h3 class="mb-1 font-medium">Test connection fails with HTTP 4xx</h3>
					<p class="text-sm text-muted-foreground">
						The connection picker probes
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>/llm/info</code
						> on the chosen URL. A 4xx response usually means the URL points at something
						other than an Eurora backend (a reverse proxy without the route configured, an
						old version, etc.).
					</p>
				</div>
			</div>
		</section>
	</div>
</div>
