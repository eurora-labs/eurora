<script module lang="ts">
	import { Login } from '$lib/custom-components/login/index.js';
	import { defineMeta, type StoryContext } from '@storybook/addon-svelte-csf';

	// eslint-disable-next-line @typescript-eslint/no-unused-vars
	const { Story } = defineMeta({
		title: 'Components / Login',
		component: Login,
		parameters: {
			docs: {
				description: {
					component:
						'A comprehensive login form component with social authentication options, form validation, and error handling. Features password visibility toggle, loading states, and integration with social auth providers.',
				},
			},
			layout: 'centered',
		},
		argTypes: {
			submitError: {
				control: { type: 'text' },
				description: 'Error message to display when form submission fails',
			},
		},
		args: {
			submitError: undefined,
		},
	});
</script>

<!-- Interactive Login Form -->
<!-- eslint-disable-next-line @typescript-eslint/no-unused-vars -->
{#snippet template(args: { submitError: string | undefined }, _context: StoryContext<typeof Story>)}
	<div class="w-full max-w-md">
		<Login
			submitError={args.submitError}
			onsubmit={(event: any) => {
				event.preventDefault();
				alert('Form submitted!');
			}}
			onApple={() => alert('Apple login clicked!')}
			onGoogle={() => alert('Google login clicked!')}
			onGitHub={() => alert('GitHub login clicked!')}
		/>
	</div>
{/snippet}
