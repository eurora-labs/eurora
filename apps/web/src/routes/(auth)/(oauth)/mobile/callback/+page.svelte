<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';

	let status = $state<'redirecting' | 'done'>('redirecting');
	let callbackUrl = $state('');

	onMount(() => {
		const params = $page.url.searchParams.toString();
		callbackUrl = `eurora://mobile/callback${params ? `?${params}` : ''}`;

		// Try to open the app via custom scheme as a fallback
		window.location.href = callbackUrl;

		// After a short delay, assume the app opened or show the success state
		setTimeout(() => {
			status = 'done';
		}, 2000);
	});
</script>

<svelte:head>
	<title>Eurora — Authentication</title>
</svelte:head>

<div
	class="flex flex-col items-center justify-center min-h-screen px-8 bg-background text-foreground"
>
	{#if status === 'redirecting'}
		<div class="flex flex-col items-center gap-4 text-center">
			<div
				class="w-10 h-10 border-4 border-muted border-t-primary rounded-full animate-spin"
			></div>
			<h1 class="text-xl font-semibold">Returning to Eurora...</h1>
			<p class="text-sm text-muted-foreground">
				You should be redirected back to the app momentarily.
			</p>
		</div>
	{:else}
		<div class="flex flex-col items-center gap-6 text-center max-w-sm">
			<svg
				xmlns="http://www.w3.org/2000/svg"
				class="w-12 h-12 text-green-500"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
				stroke-linecap="round"
				stroke-linejoin="round"
			>
				<path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
				<polyline points="22 4 12 14.01 9 11.01" />
			</svg>

			<h1 class="text-xl font-semibold">Authentication complete</h1>
			<p class="text-sm text-muted-foreground">
				You can close this tab and return to the Eurora app.
			</p>

			<a
				href={callbackUrl}
				class="inline-flex items-center justify-center rounded-md bg-primary px-6 py-2.5 text-sm font-medium text-primary-foreground shadow hover:bg-primary/90 transition-colors"
			>
				Open Eurora
			</a>

			<p class="text-xs text-muted-foreground">
				Don't have the app?
				<a
					href="/"
					class="underline underline-offset-4 hover:text-foreground transition-colors"
				>
					Learn more
				</a>
			</p>
		</div>
	{/if}
</div>
