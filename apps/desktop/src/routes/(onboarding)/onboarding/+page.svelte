<script lang="ts">
	import { Login } from '@eurora/ui/custom-components/login/index';

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
