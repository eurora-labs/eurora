<script lang="ts">
    import { onMount, onDestroy } from 'svelte';
    import { Skeleton } from '@eurora/ui';
  
    const isDev = import.meta.env.DEV;

    let {
        portalId,
        formId,
        submitText = 'Submit',
        region = 'na1'
    } = $props();
  
    // export let portalId: string,
    //   formId: string | number,
    //   submitText = 'Submit',
    //   region = 'na1';
  
    const targetElementID = `form-${formId}`;
    let formLoaded = $state(false);
  
    onMount(() => {
      if (isDev) console.log(`component:HubspotFormSvelte ${formId} mounted`);
    });
  
    const handleCreateForm = () => {
      //@ts-ignore
      if (window.hbspt) {
        //@ts-ignore
        hbspt.forms.create({
          region,
          portalId,
          formId,
          submitText,
          target: '#' + targetElementID,
          css: '', //undocumented but required for iframe styling
          onFormReady: function (form: any) {
            formLoaded = true;
            if (isDev) console.log(`component:HubspotFormSvelte ${formId} form loaded`);
          }
        });
      }
    };
  
    onDestroy(() => {
      if (isDev) console.log(`component:HubspotFormSvelte ${formId} destroyed`);
    });
  </script>


  <svelte:head>
    <script
        charset="utf-8"
        type="text/javascript"
        src="//js.hsforms.net/forms/embed/v2.js"
        onload={handleCreateForm}></script>
  </svelte:head>

  {#if !formLoaded}
    <div class="w-full space-y-2">
      <Skeleton class="h-10 w-full" />
      <Skeleton class="h-10 w-full" />
      <Skeleton class="h-10 w-full" />
      <Skeleton class="h-10 w-3/4 mx-auto" />
    </div>
  {/if}

  <div
    id={targetElementID}
    class="base-hubspot-form"
    ></div>