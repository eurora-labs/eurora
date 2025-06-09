<script lang="ts" module>
	export interface SocialAuthButtonsProps {
		mode: 'login' | 'register';
		disabled: boolean;
		onGoogle: () => void;
		onGitHub: () => void;
	}
</script>

<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { SiGoogle, SiGithub } from '@icons-pack/svelte-simple-icons';

	let { mode, disabled, onGoogle, onGitHub }: SocialAuthButtonsProps = $props();

	const buttonText = {
		login: {
			google: 'Continue with Google',
			github: 'Continue with GitHub'
		},
		register: {
			google: 'Register with Google',
			github: 'Register with GitHub'
		}
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
</div>
