<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
</script>

<div class="flex min-h-screen flex-col items-center justify-center gap-6 p-8">
	<article class="prose max-w-none">
		<h1 class="text-4xl font-bold">Download Browser Extension</h1>
		<p>In order to ensure best user experience, please download our browser extension.</p>
	</article>
	<Button onclick={() => goto('browser-extension/redirect')}>Download</Button>
</div>
