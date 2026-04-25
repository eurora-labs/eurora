<script lang="ts">
	import { USER_SERVICE } from '$lib/services/user-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Input } from '@eurora/ui/components/input/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Separator } from '@eurora/ui/components/separator/index';

	const user = inject(USER_SERVICE);
</script>

<div class="flex flex-col gap-8 px-4 py-6">
	<header class="flex flex-col gap-1">
		<h1 class="text-lg font-semibold">Settings</h1>
		<p class="text-sm text-muted-foreground">Account and application preferences.</p>
	</header>

	<section class="flex flex-col gap-4">
		<h2 class="text-sm font-medium text-muted-foreground">Account</h2>
		<Separator />
		<div class="flex flex-col gap-2">
			<Label for="account-name">Name</Label>
			<Input id="account-name" disabled value={user.displayName ?? ''} />
		</div>
		<div class="flex flex-col gap-2">
			<Label for="account-email">Email</Label>
			<Input id="account-email" disabled value={user.email} />
		</div>
	</section>
</div>
