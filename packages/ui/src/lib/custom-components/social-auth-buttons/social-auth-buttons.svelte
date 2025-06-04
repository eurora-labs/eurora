<script lang="ts">
	import { Button } from '$lib/components/button/index.js';
	import { SiApple, SiGoogle, SiGithub } from '@icons-pack/svelte-simple-icons';

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
		<SiApple />
		{buttonText[mode].apple}
	</Button>
	<Button variant="outline" class="w-full" onclick={onGoogle} {disabled}>
		<SiGoogle />
		{buttonText[mode].google}
	</Button>
	<Button variant="outline" class="w-full" onclick={onGitHub} {disabled}>
		<SiGithub />
		{buttonText[mode].github}
	</Button>
</div>
