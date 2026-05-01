<svelte:head>
	<title>Eurora - Authentication</title>
</svelte:head>

<div
	class="flex flex-col items-center justify-center min-h-screen px-8 bg-background text-foreground"
>
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

		<h1 class="text-xl font-semibold">You're signed in</h1>
		<p class="text-sm text-muted-foreground">
			Switch back to the Eurora app to continue. It'll detect the sign-in automatically within
			a couple of seconds.
		</p>

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
</div>
