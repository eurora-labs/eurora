<script lang="ts">
	import { Login } from '@eurora/ui/custom-components/login/index';
	import { Button } from '@eurora/ui/components/button/index';

	function onSubmit(event: SubmitEvent) {
		event.preventDefault();
		console.log('Form submitted');
	}

	function onApple() {
		console.log('Apple login');
	}

	function onGoogle() {
		console.log('Google login');
	}

	function onGitHub() {
		console.log('GitHub login');
	}
</script>

<Login onsubmit={onSubmit} {onApple} {onGoogle} {onGitHub} />
<Button class="mt-6" href="/onboarding">Back</Button>
