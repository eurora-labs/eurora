<script lang="ts" module>
	import type { PageProps } from '../$types';
	interface Props extends PageProps {
		data: {
			slug: string;
		};
	}
</script>

<script lang="ts">
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';

	let { data }: Props = $props();
	onMount(() => {
		sessionStorage.setItem('loginToken', data.slug);
		goto('/login');
	});
</script>
