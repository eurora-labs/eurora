<script lang="ts">
	import { TAURPC_SERVICE } from '$lib/bindings/taurpcService.js';
	import { inject } from '@eurora/shared/context';
	import { onMount } from 'svelte';
	import { toast } from 'svelte-sonner';

	const taurpcService = inject(TAURPC_SERVICE);

	let isUpdating = $state(false);

	async function checkForUpdate() {
		try {
			const updateInfo = await taurpcService.system.check_for_update();

			if (updateInfo) {
				toast.info(`Update available: v${updateInfo.version}`, {
					description: updateInfo.body ?? 'A new version is ready to install.',
					duration: Infinity,
					action: {
						label: 'Update now',
						onClick: installUpdate,
					},
					cancel: {
						label: 'Later',
						onClick: () => {},
					},
				});
			}
		} catch (error) {
			console.error('Failed to check for updates:', error);
		}
	}

	async function installUpdate() {
		if (isUpdating) return;
		isUpdating = true;

		const toastId = toast.loading('Downloading update...', {
			description: 'The app will restart when complete.',
		});

		try {
			await taurpcService.system.install_update();
			// If we get here, the app didn't restart (shouldn't happen normally)
			toast.success('Update installed!', {
				id: toastId,
				description: 'Restarting application...',
			});
		} catch (error) {
			console.error('Failed to install update:', error);
			toast.error('Update failed', {
				id: toastId,
				description: String(error),
			});
		}
		isUpdating = false;
	}

	onMount(() => {
		// Small delay to ensure the app is fully loaded before checking
		const timeout = setTimeout(checkForUpdate, 2000);
		return () => clearTimeout(timeout);
	});
</script>
