<script lang="ts">
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { open } from '@tauri-apps/plugin-shell';

	const taurpc = inject(TAURPC_SERVICE);

	async function downloadBrowserExtension() {
		const url = await taurpc.onboarding.get_browser_extension_download_url();
		await open(url);
	}
</script>

<div class="flex flex-col justify-center items-center h-full p-8 w-full">
	<div class="flex flex-col justify-center items-center w-full h-full">
		<h1 class="text-2xl font-bold mb-8">Download Browser Extension</h1>

		<Button onclick={downloadBrowserExtension}>Download</Button>
	</div>
	<div class="flex justify-between w-full items-start mt-auto pt-8">
		<Button variant="outline" href="/onboarding">Back</Button>
		<Button href="/">Continue</Button>
	</div>
</div>
