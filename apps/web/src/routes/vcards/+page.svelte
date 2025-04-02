<script lang="ts">
  import { onMount } from 'svelte';
  
  let vcards: any[] = [];
  
  onMount(async () => {
    try {
      // In a real application, you might want to fetch this list from an API
      // For now, we'll hardcode the list based on the files we know exist
      vcards = [
        { name: 'maitte_oliveira', displayName: 'Maitte Oliveira' },
        { name: 'andre_roelofs', displayName: 'Andre Roelofs' },
        { name: 'laura_thommen', displayName: 'Laura Thommen' },
        { name: 'linda_rosner', displayName: 'Linda Rosner' }
      ];
    } catch (error) {
      console.error('Error loading vCards:', error);
    }
  });
</script>

<svelte:head>
  <title>Eurora Team vCards</title>
</svelte:head>

<div class="container mx-auto px-4 py-8">
  <h1 class="text-3xl font-bold mb-6">Eurora Team vCards</h1>
  
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
    {#each vcards as vcard}
      <div class="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow">
        <h2 class="text-xl font-semibold mb-2">{vcard.displayName}</h2>
        <p class="text-gray-600 mb-4">Eurora Team Member</p>
        <a 
          href={`/vcards/${vcard.name}`} 
          class="inline-block bg-blue-600 text-white px-4 py-2 rounded hover:bg-blue-700 transition-colors"
        >
          Download vCard
        </a>
      </div>
    {/each}
  </div>
</div>