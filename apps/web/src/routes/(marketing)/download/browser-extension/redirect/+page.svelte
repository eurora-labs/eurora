<script lang="ts">
	import DownloadButton from '$lib/components/marketing/DownloadButton.svelte';
	import { Button } from '@eurora/ui/components/button/index';
	import Chromium from '@lucide/svelte/icons/chromium';
	import Globe from '@lucide/svelte/icons/globe';
	import MonitorIcon from '@lucide/svelte/icons/monitor';

	const FIREFOX_EXTENSION_URL = 'https://addons.mozilla.org/en-US/firefox/addon/eurora';
	const CHROME_EXTENSION_URL =
		'https://chromewebstore.google.com/detail/bfndcocdeinignobnnjplgoggmgebihm';
	const EDGE_EXTENSION_URL =
		'https://microsoftedge.microsoft.com/addons/detail/eurora/jldnbebjeaegfgpboohhoipokpbpncke';

	type BrowserType = 'firefox' | 'chrome' | 'edge' | 'safari' | 'unknown';

	let browserType = $state<BrowserType>('unknown');
	let redirecting = $state(false);

	function detectBrowser(): BrowserType {
		if (typeof navigator === 'undefined') {
			return 'unknown';
		}

		const ua = navigator.userAgent.toLowerCase();

		if (ua.includes('firefox')) return 'firefox';
		if (ua.includes('edg/') || ua.includes('edga/') || ua.includes('edgios/')) return 'edge';
		if (ua.includes('safari') && !ua.includes('chrome') && !ua.includes('chromium'))
			return 'safari';
		if (ua.includes('chrome') || ua.includes('chromium')) return 'chrome';

		return 'unknown';
	}

	function getRedirectUrl(browser: BrowserType): string | null {
		switch (browser) {
			case 'firefox':
				return FIREFOX_EXTENSION_URL;
			case 'chrome':
				return CHROME_EXTENSION_URL;
			case 'edge':
				return EDGE_EXTENSION_URL;
			default:
				return null;
		}
	}

	function getStoreName(browser: BrowserType): string {
		switch (browser) {
			case 'firefox':
				return 'Firefox Add-ons';
			case 'chrome':
				return 'Chrome Web Store';
			case 'edge':
				return 'Edge Add-ons';
			default:
				return '';
		}
	}

	$effect(() => {
		if (typeof window !== 'undefined') {
			browserType = detectBrowser();
			const url = getRedirectUrl(browserType);

			if (url) {
				redirecting = true;
				window.location.href = url;
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
				Redirecting to {getStoreName(browserType)}...
			</p>
		</div>
	{:else if browserType === 'safari'}
		<div class="flex flex-col items-center gap-6 text-center">
			<h1 class="text-3xl font-bold">Eurora for Safari</h1>
			<p class="max-w-md text-gray-600">
				The Safari extension is included automatically with the Eurora desktop app. Download
				the app to get started.
			</p>
			<DownloadButton />
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
				<Button
					class="bg-cyan-700 w-full"
					variant="secondary"
					size="lg"
					href={EDGE_EXTENSION_URL}
				>
					<MonitorIcon class="h-5 w-5" />
					Edge
				</Button>
			</div>
			<p class="mt-4 text-sm text-gray-500">
				For Safari, the extension is included with the
				<a href="/download" class="underline hover:text-gray-700">desktop app</a>.
			</p>
		</div>
	{/if}
</div>
