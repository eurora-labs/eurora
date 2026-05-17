<script lang="ts" module>
	export interface SocialAuthButtonsProps {
		mode: 'login' | 'register';
		disabled: boolean;
		onGoogle: () => void;
		onGitHub: () => void;
		onApple: () => void;
	}
</script>

<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { SiGoogle, SiGithub } from '@icons-pack/svelte-simple-icons';

	let { mode, disabled, onGoogle, onGitHub, onApple }: SocialAuthButtonsProps = $props();

	const buttonText = {
		login: {
			google: 'Continue with Google',
			github: 'Continue with GitHub',
			apple: 'Continue with Apple',
		},
		register: {
			google: 'Register with Google',
			github: 'Register with GitHub',
			apple: 'Register with Apple',
		},
	};
</script>

<div class="space-y-3">
	<Button variant="outline" class="w-full" onclick={onGoogle} {disabled}>
		<SiGoogle />
		{buttonText[mode].google}
	</Button>
	<Button variant="outline" class="w-full" onclick={onGitHub} {disabled}>
		<SiGithub />
		{buttonText[mode].github}
	</Button>
	<!--
		Apple sign-in hidden: backend flow not finished for release.
		Apple's logo isn't in Simple Icons (trademark-protected), and
		Apple's brand guidelines require a specific glyph + spacing for
		the Sign-in-with-Apple button. UI styling is deferred to a
		follow-up — for now we ship the functional button with an inline
		SVG that approximates the system glyph.
	-->
	<Button variant="outline" class="w-full hidden" onclick={onApple} {disabled}>
		<svg
			xmlns="http://www.w3.org/2000/svg"
			width="16"
			height="16"
			viewBox="0 0 24 24"
			fill="currentColor"
			aria-hidden="true"
		>
			<path
				d="M17.5 12.5c0-2.6 2.1-3.9 2.2-3.9-1.2-1.7-3-2-3.7-2-1.6-.2-3 .9-3.8.9s-2-.9-3.3-.9c-1.7 0-3.2 1-4.1 2.5-1.7 3-.4 7.4 1.2 9.8.8 1.2 1.8 2.5 3.1 2.4 1.2-.1 1.7-.8 3.2-.8s2 .8 3.3.8c1.4 0 2.3-1.2 3.1-2.4.7-1 1.4-2.3 1.4-2.4-.1 0-2.6-1-2.6-4zM14.7 4.2c.7-.9 1.2-2.1 1.1-3.2-1 0-2.2.7-3 1.5-.7.8-1.3 2-1.1 3.1 1.1.1 2.3-.6 3-1.4z"
			/>
		</svg>
		{buttonText[mode].apple}
	</Button>
</div>
