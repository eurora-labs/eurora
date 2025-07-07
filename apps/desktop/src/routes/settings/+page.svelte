<script lang="ts">
	import { Label } from '@eurora/ui/components/label/index';
	import { Input } from '@eurora/ui/components/input/index';
	import * as Card from '@eurora/ui/components/card/index';

	import FirstPartyLogin from '$lib/components/FirstPartyLogin.svelte';
</script>

<div class="flex flex-col p-4 gap-4">
	<h1 class="text-2xl font-semibold">General</h1>

	<FirstPartyLogin />

	<div class="flex flex-col gap-4">
		<div class="flex items-center gap-2">
			<Label for="name">Name</Label>
			<Input id="name" disabled value="Eurora" />
		</div>

		<div class="flex items-center gap-2">
			<Label for="email">Email</Label>
			<Input id="email" disabled value="Eurora@eurora.ai" />
		</div>
	</div>
</div>
