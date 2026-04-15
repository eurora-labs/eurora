<script lang="ts">
	import { goto } from '$app/navigation';
	import { CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
	import SidebarThreadsList from '@eurora/chat/components/SidebarThreadsList.svelte';
	import { inject } from '@eurora/shared/context';
	import * as Sidebar from '@eurora/ui/components/sidebar/index';
	import SquarePenIcon from '@lucide/svelte/icons/square-pen';
	import { onMount } from 'svelte';

	const chatService = inject(CHAT_SERVICE);

	let threadInitialized = false;

	$effect(() => {
		if (!threadInitialized) {
			threadInitialized = true;
			chatService.loadThreads(20, 0);
		}
	});

	onMount(() => {
		return () => {
			chatService.destroy();
		};
	});

	function handleThreadSelect(threadId: string) {
		if (threadId) {
			goto(`/${threadId}`);
		} else {
			goto('/');
		}
	}

	function createChat() {
		chatService.activeThreadId = undefined;
		goto('/');
	}
</script>

<Sidebar.Root side="left">
	<Sidebar.Header>
		<Sidebar.Menu>
			<Sidebar.MenuItem>
				<Sidebar.MenuButton onclick={createChat}>
					<SquarePenIcon />
					<span>New chat</span>
				</Sidebar.MenuButton>
			</Sidebar.MenuItem>
		</Sidebar.Menu>
	</Sidebar.Header>
	<Sidebar.Content>
		<SidebarThreadsList onThreadSelect={handleThreadSelect} />
	</Sidebar.Content>
</Sidebar.Root>
