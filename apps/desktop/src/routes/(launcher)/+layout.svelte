<script lang="ts">
	import '../../app.pcss';
	import { platform } from '@tauri-apps/plugin-os';
	let { children } = $props();

	document.body.classList.add(`${platform()}-app`);
</script>

<main class="flex-1 overflow-hidden bg-transparent">
	{@render children?.()}
</main>
