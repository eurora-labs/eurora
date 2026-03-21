<script lang="ts">
	import { page } from '$app/state';
	import { Button } from '@eurora/ui/components/button/index';
</script>

<div class="flex min-h-[60vh] flex-col items-center justify-center gap-4 px-4 text-center">
	<h1 class="text-6xl font-bold tracking-tight">{page.status}</h1>
	<p class="text-lg text-muted-foreground">{page.error?.message ?? 'Something went wrong'}</p>
	<Button href="/" variant="outline" class="mt-4">Back to Home</Button>
</div>
