<script lang="ts">
	import * as CodeBlock from '@eurora/ui/components/ai-elements/code-block/index';
	import { Alert, AlertDescription } from '@eurora/ui/components/alert/index';
	import InfoIcon from '@lucide/svelte/icons/info';
	import TriangleAlertIcon from '@lucide/svelte/icons/triangle-alert';

	const ollamaPullCommand = 'ollama pull llama3.2';
	const ollamaServeCommand = 'ollama serve';

	const envFileContents = `# Required — set these to random strings (e.g. openssl rand -hex 32)
JWT_ACCESS_SECRET=
JWT_REFRESH_SECRET=

# Optional — uncomment and change as needed
# OLLAMA_MODEL=llama3.2
# EURORA_GRPC_PORT=39051
# EURORA_POSTGRES_PORT=39432
# EURORA_ASSET_DIR=~/.local/share/eurora/assets
# APPROVED_EMAILS=*`;

	const composeFile = `services:
  postgres:
    image: postgres:16
    restart: unless-stopped
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: eurora
    ports:
      - '\${EURORA_POSTGRES_PORT:-39432}:5432'
    volumes:
      - eurora-pgdata:/var/lib/postgresql/data
    healthcheck:
      test: ['CMD-SHELL', 'pg_isready -U postgres']
      interval: 5s
      timeout: 3s
      retries: 5

  backend:
    image: ghcr.io/eurora-labs/eurora/be-monolith:latest
    restart: unless-stopped
    depends_on:
      postgres:
        condition: service_healthy
    ports:
      - '\${EURORA_GRPC_PORT:-39051}:\${EURORA_GRPC_PORT:-39051}'
    extra_hosts:
      - 'host.docker.internal:host-gateway'
    user: '\${EURORA_UID:-1000}:\${EURORA_GID:-1000}'
    environment:
      REMOTE_DATABASE_URL: postgresql://postgres:postgres@postgres:5432/eurora
      MONOLITH_ADDR: 0.0.0.0:\${EURORA_GRPC_PORT:-39051}
      RUNNING_EURORA_FULLY_LOCAL: 'true'
      OLLAMA_MODEL: \${OLLAMA_MODEL:-llama3.2}
      OLLAMA_HOST: http://host.docker.internal:11434
      JWT_ACCESS_SECRET: \${JWT_ACCESS_SECRET:-change-me-access}
      JWT_REFRESH_SECRET: \${JWT_REFRESH_SECRET:-change-me-refresh}
      ASSET_STORAGE_BACKEND: fs
      ASSET_STORAGE_FS_ROOT: /data/assets
      APPROVED_EMAILS: '\${APPROVED_EMAILS:-*}'
    volumes:
      - \${EURORA_ASSET_DIR:-~/.local/share/eurora/assets}:/data/assets

volumes:
  eurora-pgdata:`;

	const startCommand = 'docker compose up -d';
	const logsCommand = 'docker compose logs -f backend';

	const envVars = [
		{
			name: 'JWT_ACCESS_SECRET',
			default: 'change-me-access',
			description: 'Secret used to sign access tokens. Must be set to a long random string.',
			required: true,
		},
		{
			name: 'JWT_REFRESH_SECRET',
			default: 'change-me-refresh',
			description: 'Secret used to sign refresh tokens. Must be set to a long random string.',
			required: true,
		},
		{
			name: 'OLLAMA_MODEL',
			default: 'llama3.2',
			description: 'The Ollama model the backend will use for inference.',
			required: false,
		},
		{
			name: 'EURORA_GRPC_PORT',
			default: '39051',
			description: 'Port the backend listens on. Change if 39051 is already in use.',
			required: false,
		},
		{
			name: 'EURORA_POSTGRES_PORT',
			default: '39432',
			description: 'Host port for PostgreSQL. Only relevant if you need direct DB access.',
			required: false,
		},
		{
			name: 'EURORA_UID / EURORA_GID',
			default: '1000',
			description:
				'User/group ID the backend container runs as. Match your host user to avoid file-permission issues.',
			required: false,
		},
		{
			name: 'EURORA_ASSET_DIR',
			default: '~/.local/share/eurora/assets',
			description: 'Host directory where uploaded assets are stored.',
			required: false,
		},
		{
			name: 'APPROVED_EMAILS',
			default: '*',
			description:
				'Comma-separated list of email addresses allowed to sign up. Use * to allow everyone.',
			required: false,
		},
	];
</script>

<div>
	<h1 class="mb-4 text-4xl font-bold">Self-Hosting</h1>
	<p class="mb-12 text-lg text-muted-foreground">
		Run the Eurora backend on your own machine using Docker Compose. This gives you full control
		over your data and lets you use local models via Ollama.
	</p>

	<div class="flex flex-col gap-12">
		<section>
			<h2 class="mb-2 text-2xl font-semibold">Prerequisites</h2>
			<ul class="list-disc space-y-2 pl-5 text-muted-foreground">
				<li>
					<strong>Docker &amp; Docker Compose</strong> — follow the
					<a
						href="https://docs.docker.com/get-docker/"
						target="_blank"
						rel="noopener noreferrer"
						class="text-primary underline underline-offset-4 hover:text-primary/80"
					>
						official installation guide</a
					>. Docker Desktop includes Compose by default.
				</li>
				<li>
					<strong>Ollama</strong> — install from
					<a
						href="https://ollama.com"
						target="_blank"
						rel="noopener noreferrer"
						class="text-primary underline underline-offset-4 hover:text-primary/80"
					>
						ollama.com</a
					>, then pull a model and make sure Ollama is running:
					<div class="mt-3 flex flex-col gap-2">
						<CodeBlock.Root code={ollamaPullCommand} language="shellscript">
							<CodeBlock.Header>
								<CodeBlock.Actions class="ml-auto">
									<CodeBlock.CopyButton />
								</CodeBlock.Actions>
							</CodeBlock.Header>
						</CodeBlock.Root>
						<CodeBlock.Root code={ollamaServeCommand} language="shellscript">
							<CodeBlock.Header>
								<CodeBlock.Actions class="ml-auto">
									<CodeBlock.CopyButton />
								</CodeBlock.Actions>
							</CodeBlock.Header>
						</CodeBlock.Root>
					</div>
					<p class="mt-2 text-sm">
						You can use any model Ollama supports — just set
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>OLLAMA_MODEL</code
						>
						in your
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code>
						file to match. If Ollama is already running in the background, you can skip
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>ollama serve</code
						>.
					</p>
				</li>
			</ul>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">1. Create a project directory</h2>
			<p class="mb-3 text-muted-foreground">
				Create a new directory for your Eurora self-hosted setup. All files from the
				following steps go in this directory.
			</p>
			<CodeBlock.Root
				code="mkdir eurora-selfhosted && cd eurora-selfhosted"
				language="shellscript"
			>
				<CodeBlock.Header>
					<CodeBlock.Actions class="ml-auto">
						<CodeBlock.CopyButton />
					</CodeBlock.Actions>
				</CodeBlock.Header>
			</CodeBlock.Root>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">2. Create a .env file</h2>
			<p class="mb-3 text-muted-foreground">
				Create a <code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code>
				file next to your compose file. At minimum, set the two JWT secrets to random values.
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
						The JWT secrets <strong>must</strong> be set before starting the services. Leaving
						them at their defaults is insecure, even for local use.
					</p>
				</AlertDescription>
			</Alert>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">3. Create docker-compose.yml</h2>
			<p class="mb-3 text-muted-foreground">
				Create a
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
					>docker-compose.yml</code
				>
				file with the following contents. Docker Compose will automatically read variables from
				your <code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code> file.
			</p>
			<CodeBlock.Root code={composeFile} language="yaml">
				<CodeBlock.Header>
					<CodeBlock.Filename>docker-compose.yml</CodeBlock.Filename>
					<CodeBlock.Actions>
						<CodeBlock.CopyButton />
					</CodeBlock.Actions>
				</CodeBlock.Header>
			</CodeBlock.Root>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">4. Start the services</h2>
			<p class="mb-3 text-muted-foreground">
				Make sure Ollama is running, then start the containers. Docker will pull the images
				automatically on the first run.
			</p>
			<CodeBlock.Root code={startCommand} language="shellscript">
				<CodeBlock.Header>
					<CodeBlock.Actions class="ml-auto">
						<CodeBlock.CopyButton />
					</CodeBlock.Actions>
				</CodeBlock.Header>
			</CodeBlock.Root>
			<p class="mt-3 mb-3 text-muted-foreground">
				To verify everything started correctly, check the logs:
			</p>
			<CodeBlock.Root code={logsCommand} language="shellscript">
				<CodeBlock.Header>
					<CodeBlock.Actions class="ml-auto">
						<CodeBlock.CopyButton />
					</CodeBlock.Actions>
				</CodeBlock.Header>
			</CodeBlock.Root>
		</section>

		<section>
			<h2 class="mb-2 text-2xl font-semibold">5. Connect the desktop app</h2>
			<p class="mb-3 text-muted-foreground">
				Open the Eurora desktop app and go to <strong>Settings &rarr; API Settings</strong>.
				Select <strong>Ollama</strong> as the provider, set the endpoint to
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
					>http://localhost:39051</code
				>
				(or your custom
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
					>EURORA_GRPC_PORT</code
				>), and click <strong>Connect</strong>.
			</p>
			<Alert>
				<InfoIcon />
				<AlertDescription>
					<p>
						If you see "Run Locally" on the onboarding screen, that link brings you to
						this guide. After Docker is running, configure the connection in
						<strong>Settings &rarr; API Settings</strong>.
					</p>
				</AlertDescription>
			</Alert>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Environment variable reference</h2>
			<p class="mb-3 text-muted-foreground">
				All variables are set in your
				<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code> file. Docker
				Compose substitutes them into the compose file automatically.
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
				<span class="text-destructive">*</span> Required — must be explicitly set.
			</p>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Troubleshooting</h2>
			<div class="flex flex-col gap-4">
				<div>
					<h3 class="mb-1 font-medium">Backend can't reach Ollama</h3>
					<p class="text-sm text-muted-foreground">
						The backend connects to Ollama via
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>host.docker.internal:11434</code
						>. Make sure Ollama is running on your host machine before starting the
						containers. On Linux, if
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>host.docker.internal</code
						>
						doesn't resolve, ensure you're using Docker 20.10+ which supports the
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>host-gateway</code
						> extra host.
					</p>
				</div>
				<div>
					<h3 class="mb-1 font-medium">Permission denied on asset directory</h3>
					<p class="text-sm text-muted-foreground">
						The container runs as
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>EURORA_UID:EURORA_GID</code
						>
						(default 1000:1000). Make sure the asset directory on the host is owned by the
						same user, or adjust the UID/GID in your
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code> file.
					</p>
				</div>
				<div>
					<h3 class="mb-1 font-medium">Port already in use</h3>
					<p class="text-sm text-muted-foreground">
						If port 39051 or 39432 conflicts with another service, change
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>EURORA_GRPC_PORT</code
						>
						or
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm"
							>EURORA_POSTGRES_PORT</code
						>
						in your
						<code class="rounded bg-muted px-1.5 py-0.5 font-mono text-sm">.env</code> file.
					</p>
				</div>
			</div>
		</section>
	</div>
</div>
