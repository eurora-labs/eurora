<script module lang="ts">
	import { Login } from '$lib/custom-components/login/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / Login',
		component: Login,
		parameters: {
			docs: {
				description: {
					component:
						'A comprehensive showcase of all login component states and configurations. Demonstrates error states, social authentication options, and various use cases.',
				},
			},
			controls: { disable: true },
		},
	});
</script>

<Story name="All Login Examples">
	{#snippet children()}
		<div class="space-y-8 p-6">
			<div>
				<h2 class="text-lg font-semibold mb-4 text-white">Default Login Form</h2>
				<div class="w-full max-w-md">
					<Login
						submitError={undefined}
						onsubmit={(event) => {
							event.preventDefault();
							console.log('Form submitted');
						}}
						onApple={() => console.log('Apple login')}
						onGoogle={() => console.log('Google login')}
						onGitHub={() => console.log('GitHub login')}
					/>
				</div>
			</div>

			<div>
				<h2 class="text-lg font-semibold mb-4 text-white">With Error State</h2>
				<div class="w-full max-w-md">
					<Login
						submitError="Invalid username or password. Please try again."
						onsubmit={(event) => {
							event.preventDefault();
							console.log('Form submitted');
						}}
						onApple={() => console.log('Apple login')}
						onGoogle={() => console.log('Google login')}
						onGitHub={() => console.log('GitHub login')}
					/>
				</div>
			</div>

			<div>
				<h2 class="text-lg font-semibold mb-4 text-white">With Network Error</h2>
				<div class="w-full max-w-md">
					<Login
						submitError="Unable to connect to the server. Please check your internet connection and try again."
						onsubmit={(event) => {
							event.preventDefault();
							console.log('Form submitted');
						}}
						onApple={() => console.log('Apple login')}
						onGoogle={() => console.log('Google login')}
						onGitHub={() => console.log('GitHub login')}
					/>
				</div>
			</div>

			<div>
				<h2 class="text-lg font-semibold mb-4 text-white">With Account Locked Error</h2>
				<div class="w-full max-w-md">
					<Login
						submitError="Your account has been temporarily locked due to multiple failed login attempts. Please try again in 15 minutes."
						onsubmit={(event) => {
							event.preventDefault();
							console.log('Form submitted');
						}}
						onApple={() => console.log('Apple login')}
						onGoogle={() => console.log('Google login')}
						onGitHub={() => console.log('GitHub login')}
					/>
				</div>
			</div>

			<div>
				<h2 class="text-lg font-semibold mb-4 text-white">Social Auth Only (No Form)</h2>
				<div class="w-full max-w-md">
					<div class="p-6 border rounded-lg bg-card">
						<div class="space-y-4">
							<h3 class="text-lg font-semibold text-center">
								Sign in to your account
							</h3>
							<div class="space-y-3">
								<button
									class="w-full flex items-center justify-center gap-2 px-4 py-2 border border-gray-300 rounded-md shadow-sm bg-white text-sm font-medium text-gray-700 hover:bg-gray-50"
									onclick={() => console.log('Apple login')}
								>
									<svg class="w-5 h-5" viewBox="0 0 24 24" fill="currentColor">
										<path
											d="M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.82-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.81-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z"
										/>
									</svg>
									Continue with Apple
								</button>
								<button
									class="w-full flex items-center justify-center gap-2 px-4 py-2 border border-gray-300 rounded-md shadow-sm bg-white text-sm font-medium text-gray-700 hover:bg-gray-50"
									onclick={() => console.log('Google login')}
								>
									<svg class="w-5 h-5" viewBox="0 0 24 24">
										<path
											fill="#4285F4"
											d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
										/>
										<path
											fill="#34A853"
											d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
										/>
										<path
											fill="#FBBC05"
											d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
										/>
										<path
											fill="#EA4335"
											d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
										/>
									</svg>
									Continue with Google
								</button>
								<button
									class="w-full flex items-center justify-center gap-2 px-4 py-2 border border-gray-300 rounded-md shadow-sm bg-white text-sm font-medium text-gray-700 hover:bg-gray-50"
									onclick={() => console.log('GitHub login')}
								>
									<svg class="w-5 h-5" fill="currentColor" viewBox="0 0 24 24">
										<path
											d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"
										/>
									</svg>
									Continue with GitHub
								</button>
							</div>
						</div>
					</div>
				</div>
			</div>

			<div>
				<h2 class="text-lg font-semibold mb-4 text-white">Responsive Layout</h2>
				<div class="grid grid-cols-1 md:grid-cols-2 gap-6">
					<div class="w-full max-w-sm">
						<h3 class="text-sm font-medium mb-2 text-gray-300">Mobile Size</h3>
						<Login
							submitError={undefined}
							onsubmit={(event) => {
								event.preventDefault();
								console.log('Mobile form submitted');
							}}
							onApple={() => console.log('Apple login')}
							onGoogle={() => console.log('Google login')}
							onGitHub={() => console.log('GitHub login')}
						/>
					</div>
					<div class="w-full max-w-md">
						<h3 class="text-sm font-medium mb-2 text-gray-300">Desktop Size</h3>
						<Login
							submitError={undefined}
							onsubmit={(event) => {
								event.preventDefault();
								console.log('Desktop form submitted');
							}}
							onApple={() => console.log('Apple login')}
							onGoogle={() => console.log('Google login')}
							onGitHub={() => console.log('GitHub login')}
						/>
					</div>
				</div>
			</div>

			<div>
				<h2 class="text-lg font-semibold mb-4 text-white">Usage Examples</h2>
				<div class="space-y-4">
					<div class="text-sm text-gray-300">
						<p class="mb-2">
							<strong>Default State:</strong> Clean form ready for user input
						</p>
						<p class="mb-2">
							<strong>Error States:</strong> Various error messages for different failure
							scenarios
						</p>
						<p class="mb-2">
							<strong>Social Authentication:</strong> Multiple OAuth providers for quick
							sign-in
						</p>
						<p class="mb-2">
							<strong>Form Validation:</strong> Client-side validation with real-time feedback
						</p>
						<p class="mb-2">
							<strong>Responsive Design:</strong> Adapts to different screen sizes
						</p>
					</div>
				</div>
			</div>
		</div>
	{/snippet}
</Story>
