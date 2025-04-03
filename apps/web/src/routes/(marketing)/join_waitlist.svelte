<script lang="ts">
    import { Sheet, ScrollArea, Skeleton } from '@eurora/ui';
    import { buttonVariants } from '@eurora/ui';
    import WaitlistForm from './waitlist_form.svelte';

</script>

<Sheet.Root>
    <Sheet.Trigger class={buttonVariants({ variant: "default" })}
      >Enterprise Contact</Sheet.Trigger
    >
    <Sheet.Content side="right">
     
      <ScrollArea class="h-screen">
        <WaitlistForm portalId="242150186" formId="f0b52ee4-94ab-477c-9ac5-a13cb3086f9b" region="na2" />
      </ScrollArea>
      
      <Sheet.Footer>
        <Skeleton class="w-full h-screen" />
        <Skeleton class="w-full h-screen" />
        <Skeleton class="w-full h-screen" />
       
      </Sheet.Footer>
    </Sheet.Content>
  </Sheet.Root>