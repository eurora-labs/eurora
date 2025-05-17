<script lang="ts">
	import { platform } from '@tauri-apps/plugin-os';
	let { children } = $props();

	document.body.classList.add(`${platform()}-app`);
</script>

{@render children?.()}
