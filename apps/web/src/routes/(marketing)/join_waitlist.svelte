<script lang="ts">
	import { ScrollArea } from '@eurora/ui/components/scroll-area/index';
	import * as Sheet from '@eurora/ui/components/sheet/index';
	import { Skeleton } from '@eurora/ui/components/skeleton/index';
	import { buttonVariants } from '@eurora/ui/components/button/index';
	import WaitlistForm from './waitlist_form.svelte';
</script>

<Sheet.Root>
	<Sheet.Trigger class={buttonVariants({ variant: 'default' })}>Enterprise Contact</Sheet.Trigger>
	<Sheet.Content side="right">
		<ScrollArea class="h-screen">
			<WaitlistForm
				portalId="242150186"
				formId="f0b52ee4-94ab-477c-9ac5-a13cb3086f9b"
				region="na2"
			/>
		</ScrollArea>

		<Sheet.Footer>
			<Skeleton class="h-screen w-full" />
			<Skeleton class="h-screen w-full" />
			<Skeleton class="h-screen w-full" />
		</Sheet.Footer>
	</Sheet.Content>
</Sheet.Root>
