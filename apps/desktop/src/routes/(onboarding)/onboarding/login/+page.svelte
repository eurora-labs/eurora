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

<div class="flex flex-col items-center gap-8">
	<div class="flex justify-center pt-0 pb-0">
		<h1 class="text-4xl font-bold drop-shadow-lg">Log in or Sign up</h1>
	</div>
	<Login class="max-w-sm" onsubmit={onSubmit} {onApple} {onGoogle} {onGitHub} />
	<!-- Align button to the left end -->
	<Button class="self-start" href="/onboarding">Back</Button>
</div>
