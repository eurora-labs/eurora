<script lang="ts">
	import '$styles/styles.css';
	import * as Alert from '@eurora/ui/components/alert/index';
	import { Button } from '@eurora/ui/components/button/index';
	import XIcon from '@lucide/svelte/icons/x';
	import { ModeWatcher, setMode } from 'mode-watcher';
	import { onMount } from 'svelte';

	onMount(() => {
		setMode('dark');
	});

	let { children } = $props();
	let showAlert = $state(true);
</script>

<ModeWatcher defaultMode="dark" track={false} />

<Alert.Root
	hidden={!showAlert}
	variant="destructive"
	class="fixed bottom-0 left-1/2 w-1/2 -translate-x-1/2 z-1"
>
	<Button
		variant="link"
		size="icon"
		class="absolute right-1 top-1"
		onclick={() => {
			showAlert = false;
		}}
	>
		<XIcon class="h-4 w-4" />
	</Button>
	<Alert.Title>Development Notice</Alert.Title>
	<Alert.Description>
		This website is still under active development. All the information avalaible is subject to
		change. Until this notice is lifted, all content of the website is to be considered
		placeholder.
	</Alert.Description>
</Alert.Root>

<div class="mx-0 w-full h-full">
	{@render children?.()}
</div>
