<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import Chromium from '@lucide/svelte/icons/chromium';
	import Globe from '@lucide/svelte/icons/globe';

	const FIREFOX_EXTENSION_URL = 'https://addons.mozilla.org/en-US/firefox/addon/eurora';
	const CHROME_EXTENSION_URL =
		'https://chromewebstore.google.com/detail/google-translate/odjnhjhlbmfmcaolcklpmhhlblkgjban';

	type BrowserType = 'firefox' | 'chromium' | 'unknown';

	let browserType = $state<BrowserType>('unknown');
	let redirecting = $state(false);

	function detectBrowser(): BrowserType {
		if (typeof navigator === 'undefined') {
			return 'unknown';
		}

		const userAgent = navigator.userAgent.toLowerCase();

		if (userAgent.includes('firefox')) {
			return 'firefox';
		}

		if (userAgent.includes('chrome') || userAgent.includes('chromium')) {
			return 'chromium';
		}

		return 'unknown';
	}

	function redirectToExtension(browser: BrowserType) {
		if (browser === 'firefox') {
			window.location.href = FIREFOX_EXTENSION_URL;
		} else if (browser === 'chromium') {
			window.location.href = CHROME_EXTENSION_URL;
		}
	}

	$effect(() => {
		if (typeof window !== 'undefined') {
			browserType = detectBrowser();

			if (browserType !== 'unknown') {
				redirecting = true;
				redirectToExtension(browserType);
			}
		}
	});
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-6 p-8">
	{#if redirecting}
		<div class="flex flex-col items-center gap-4">
			<div
				class="h-8 w-8 animate-spin rounded-full border-4 border-gray-300 border-t-blue-600"
			></div>
			<p class="text-lg text-gray-600">
				Redirecting to {browserType === 'firefox'
					? 'Firefox Add-ons'
					: 'Chrome Web Store'}...
			</p>
		</div>
	{:else}
		<div class="flex flex-col items-center gap-6 text-center">
			<h1 class="text-3xl font-bold">Eurora Browser Extension</h1>
			<p class="max-w-md text-gray-600">
				We couldn't automatically detect your browser. Please choose your browser below to
				install the extension:
			</p>
			<div class="flex flex-wrap justify-center gap-4">
				<Button
					class="bg-red-600 w-full"
					variant="secondary"
					size="lg"
					href={FIREFOX_EXTENSION_URL}
				>
					<Globe class="h-5 w-5" />
					Firefox
				</Button>
				<Button
					class="bg-blue-600 w-full"
					variant="secondary"
					size="lg"
					href={CHROME_EXTENSION_URL}
				>
					<Chromium class="h-5 w-5" />
					Chrome / Chromium
				</Button>
			</div>
			<p class="mt-4 text-sm text-gray-500">
				Works with Chrome, Edge, Brave, Opera, Vivaldi, and other Chromium-based browsers
			</p>
		</div>
	{/if}
</div>
