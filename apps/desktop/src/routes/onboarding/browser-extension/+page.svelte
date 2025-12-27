<script lang="ts">
	import { goto } from '$app/navigation';
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { open } from '@tauri-apps/plugin-shell';

	const taurpc = inject(TAURPC_SERVICE);

	async function downloadBrowserExtension() {
		const url = await taurpc.onboarding.get_browser_extension_download_url();
		await open(url);
		goto('fallback');
	}
</script>

<div class="w-full h-full mx-auto p-6 flex flex-col">
	<h1 class="text-2xl font-bold mb-8">Download Browser Extension</h1>

	<Button onclick={downloadBrowserExtension}>Download</Button>

	<div class="flex justify-between items-end mt-auto pt-8">
		<Button variant="outline" href="/onboarding">Back</Button>
	</div>
</div>
