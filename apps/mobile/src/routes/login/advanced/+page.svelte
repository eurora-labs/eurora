<script lang="ts">
	import { goto } from '$app/navigation';
	import { Button } from '@eurora/ui/components/button/index';
	import ArrowLeft from '@lucide/svelte/icons/arrow-left';
</script>

<div
	class="flex h-full flex-col px-6 pt-[env(safe-area-inset-top)] pb-[max(env(safe-area-inset-bottom),1.5rem)]"
>
	<header class="flex shrink-0 items-center py-2">
		<Button variant="ghost" size="icon" aria-label="Back" onclick={() => goto('/login')}>
			<ArrowLeft />
		</Button>
	</header>

	<div class="flex flex-1 items-center justify-center">
		<p class="text-sm text-muted-foreground">This is advanced placeholder</p>
	</div>
</div>
