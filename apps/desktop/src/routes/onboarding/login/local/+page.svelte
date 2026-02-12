<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Input } from '@eurora/ui/components/input/index';
	import * as Item from '@eurora/ui/components/item/index';
	import CheckIcon from '@lucide/svelte/icons/check';
	import LoaderIcon from '@lucide/svelte/icons/loader';
	import PlayIcon from '@lucide/svelte/icons/play';
	import { toast } from 'svelte-sonner';

	let taurpc = inject(TAURPC_SERVICE);

	let ollamaModel = $state('llama3.2');
	let starting = $state(false);
	let registering = $state(false);
	let backendInfo: { grpc_port: number; http_port: number; postgres_port: number } | null =
		$state(null);

	let username = $state('local');
	let email = $state('local@localhost');
	let password = $state('local');

	async function startBackend() {
		starting = true;
		try {
			const info = await taurpc.system.start_local_backend(ollamaModel);
			backendInfo = info;
			toast.success(`Backend started on gRPC :${info.grpc_port}, HTTP :${info.http_port}`);
		} catch (error) {
			toast.error(`Failed to start backend: ${error}`);
		} finally {
			starting = false;
		}
	}

	async function registerAndEnter() {
		registering = true;
		try {
			try {
				await taurpc.auth.register(username, email, password);
			} catch (registerError) {
				const regMsg = String(registerError);
				if (!regMsg.includes('already exists')) {
					throw registerError;
				}
				try {
					await taurpc.auth.login(email, password);
				} catch (loginError) {
					throw new Error(`Registration failed: ${regMsg}\nLogin failed: ${loginError}`);
				}
			}
			toast.success('Logged in to local backend');
			goto('/');
		} catch (error) {
			toast.error(`Authentication failed: ${error}`);
		} finally {
			registering = false;
		}
	}
</script>

<div class="flex flex-col justify-center items-start h-full p-8">
	<h1 class="text-4xl font-bold drop-shadow-lg pb-4">Run Locally</h1>
	<p class="text-muted-foreground pb-6">
		Set up the Eurora backend on your own machine using Docker.
	</p>

	<div class="w-full flex-1 overflow-y-auto pb-8">
		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>1. Make sure Docker is installed and running</Item.Title>
				<Item.Description>
					You can download Docker from the
					<a href="https://docs.docker.com/get-docker/" target="_blank"
						>official website</a
					>.
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>2. Choose your Ollama model</Item.Title>
				<Item.Description class="line-clamp-none">
					<span>
						Enter the name of an Ollama model running on your machine (e.g.
						<code>llama3.2</code>, <code>mistral</code>, <code>gemma3</code>).
					</span>
					<Input
						class="mt-2 font-mono text-sm"
						placeholder="llama3.2"
						bind:value={ollamaModel}
						disabled={!!backendInfo}
					/>
				</Item.Description>
			</Item.Content>
		</Item.Root>

		<Item.Root variant="default">
			<Item.Content>
				<Item.Title>3. Start the backend</Item.Title>
				<Item.Description>
					This will start the Eurora backend and a PostgreSQL database using Docker
					Compose.
				</Item.Description>
			</Item.Content>
			<Item.Actions>
				<Button onclick={startBackend} disabled={starting || !!backendInfo}>
					{#if starting}
						<LoaderIcon class="size-4 animate-spin" />
						Starting...
					{:else if backendInfo}
						<CheckIcon class="size-4" />
						Started
					{:else}
						<PlayIcon class="size-4" />
						Start
					{/if}
				</Button>
			</Item.Actions>
		</Item.Root>

		{#if backendInfo}
			<Item.Root variant="default">
				<Item.Content>
					<Item.Title>4. Create local account</Item.Title>
					<Item.Description class="line-clamp-none">
						<span>Set up a local account to use with the backend.</span>
						<div class="mt-2 flex flex-col gap-2">
							<Input
								class="font-mono text-sm"
								placeholder="Username"
								bind:value={username}
								disabled={registering}
							/>
							<Input
								class="font-mono text-sm"
								placeholder="Email"
								type="email"
								bind:value={email}
								disabled={registering}
							/>
							<Input
								class="font-mono text-sm"
								placeholder="Password"
								type="password"
								bind:value={password}
								disabled={registering}
							/>
						</div>
					</Item.Description>
				</Item.Content>
				<Item.Actions>
					<Button onclick={registerAndEnter} disabled={registering}>
						{#if registering}
							<LoaderIcon class="size-4 animate-spin" />
							Signing in...
						{:else}
							<PlayIcon class="size-4" />
							Sign in & Enter
						{/if}
					</Button>
				</Item.Actions>
			</Item.Root>
		{/if}
	</div>

	<div class="pt-8">
		<Button variant="default" onclick={() => goto('/onboarding/no-access')}>Back</Button>
	</div>
</div>
