<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import MailIcon from '@lucide/svelte/icons/mail';
</script>

<svelte:head>
	<title>Verify Email - Eurora Labs</title>
</svelte:head>

<div class="flex min-h-screen items-center justify-center px-4">
	<div class="w-full max-w-md space-y-8">
		<Card.Root class="p-6">
			<div class="space-y-4 text-center">
				<div
					class="mx-auto flex h-12 w-12 items-center justify-center rounded-full bg-blue-100"
				>
					<MailIcon class="h-6 w-6 text-blue-600" />
				</div>
				<h2 class="text-xl font-semibold">Check your email</h2>
				<p class="text-muted-foreground">
					We've sent a verification link to your email address. Please check your inbox
					and click the link to verify your account.
				</p>
				<Button variant="outline" href="/login" class="mt-4">Back to Sign In</Button>
			</div>
		</Card.Root>
	</div>
</div>
