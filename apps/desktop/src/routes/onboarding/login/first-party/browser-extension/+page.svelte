<script lang="ts">
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { open } from '@tauri-apps/plugin-shell';
	import { platform } from '@tauri-apps/plugin-os';

	const isMacos = platform() === 'macos';
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

		{#if isMacos}
			<div class="mt-6 max-w-md text-sm text-muted-foreground">
				<p class="font-medium text-foreground">Using Safari?</p>
				<p class="mt-1">After downloading, you'll need to enable the extension manually:</p>
				<ol class="mt-2 list-decimal list-inside space-y-1">
					<li>
						Open <span class="font-medium">Safari</span> and go to
						<span class="font-medium">Settings</span> (⌘,)
					</li>
					<li>Click the <span class="font-medium">Extensions</span> tab</li>
					<li>
						Find <span class="font-medium">Eurora</span> in the list and check the box to
						enable it
					</li>
					<li>
						When prompted, click <span class="font-medium">"Turn On"</span> to confirm
					</li>
				</ol>
			</div>
		{/if}
	</div>
	<div class="flex justify-between w-full items-start mt-auto pt-8">
		<Button variant="outline" href="/onboarding">Back</Button>
		<Button href="/">Continue</Button>
	</div>
</div>
