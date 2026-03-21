<script lang="ts">
	import OAuthCallback from '$lib/components/OAuthCallback.svelte';
	import { Provider } from '@eurora/shared/proto/auth_service_pb.js';
</script>

<OAuthCallback provider={Provider.GITHUB} />
