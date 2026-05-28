<script lang="ts">
	import { Button } from '$lib/components/button/index.js';
	import IconBrandApple from '@tabler/icons-svelte-runes/icons/brand-apple';
	import IconBrandGithub from '@tabler/icons-svelte-runes/icons/brand-github';
	import IconBrandGoogle from '@tabler/icons-svelte-runes/icons/brand-google';

	export interface Props {
		mode: 'login' | 'register';
		disabled: boolean;
		onApple: () => void;
		onGoogle: () => void;
		onGitHub: () => void;
	}

	let { mode, disabled, onApple, onGoogle, onGitHub }: Props = $props();

	const buttonText = {
		login: {
			apple: 'Continue with Apple',
			google: 'Continue with Google',
			github: 'Continue with GitHub',
		},
		register: {
			apple: 'Register with Apple',
			google: 'Register with Google',
			github: 'Register with GitHub',
		},
	};
</script>

<div class="space-y-3">
	<Button variant="outline" class="w-full" onclick={onApple} {disabled}>
		<IconBrandApple />
		{buttonText[mode].apple}
	</Button>
	<Button variant="outline" class="w-full" onclick={onGoogle} {disabled}>
		<IconBrandGoogle />
		{buttonText[mode].google}
	</Button>
	<Button variant="outline" class="w-full" onclick={onGitHub} {disabled}>
		<IconBrandGithub />
		{buttonText[mode].github}
	</Button>
</div>
