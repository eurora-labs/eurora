<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import EuroraIcon from '@eurora/ui/custom-icons/EuroraLogo.svelte';
	import { SiGithub } from '@icons-pack/svelte-simple-icons';
	import GlobeIcon from '@lucide/svelte/icons/globe';
</script>

<div
	class="bg-background text-foreground flex min-h-[300px] min-w-[600px] flex-col items-center justify-between rounded-lg p-6"
>
	<!-- Header with Logo -->
	<div class="mt-6 flex flex-col items-center">
		<div class="relative">
			<div
				class="from-primary/20 to-secondary/20 absolute -inset-1 rounded-full bg-gradient-to-r blur-md"
			></div>
			<EuroraIcon class="relative mb-2 h-28 w-28" />
		</div>
		<h1
			class="from-primary to-secondary mt-3 bg-gradient-to-r bg-clip-text text-3xl font-bold text-transparent"
		>
			Eurora Labs
		</h1>
	</div>

	<div class="flex w-full flex-grow items-center justify-center"></div>

	<Separator />

	<!-- Footer with Icons -->
	<div class="mt-6 flex w-full items-center justify-center gap-6 pt-4">
		<Button
			variant="ghost"
			size="icon"
			href="https://eurora-labs.com"
			class="hover:bg-muted rounded-full transition-colors"
		>
			<div class="text-muted-foreground hover:text-foreground h-5 w-5">
				<GlobeIcon size={20} />
			</div>
		</Button>
		<Button
			variant="ghost"
			size="icon"
			href="https://github.com/Eurora-Labs/eurora"
			class="hover:bg-muted rounded-full transition-colors"
		>
			<div class="text-muted-foreground hover:text-foreground h-5 w-5">
				<SiGithub size={20} />
			</div>
		</Button>
	</div>
</div>
