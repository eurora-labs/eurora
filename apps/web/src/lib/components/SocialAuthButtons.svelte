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
	import IconBrandApple from '@tabler/icons-svelte-runes/icons/brand-apple';
	import IconBrandGithub from '@tabler/icons-svelte-runes/icons/brand-github';
	import IconBrandGoogle from '@tabler/icons-svelte-runes/icons/brand-google';

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
		<IconBrandGoogle />
		{buttonText[mode].google}
	</Button>
	<Button variant="outline" class="w-full" onclick={onGitHub} {disabled}>
		<IconBrandGithub />
		{buttonText[mode].github}
	</Button>
	<!-- Apple sign-in hidden: backend flow not finished for release. -->
	<Button variant="outline" class="w-full hidden" onclick={onApple} {disabled}>
		<IconBrandApple />
		{buttonText[mode].apple}
	</Button>
</div>
