<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import * as DropdownMenu from '@eurora/ui/components/dropdown-menu/index';
	import * as Avatar from '@eurora/ui/components/avatar/index';
	import LogOutIcon from '@lucide/svelte/icons/log-out';
	import UserIcon from '@lucide/svelte/icons/user';
	import { currentUser, auth } from '$lib/stores/auth.js';
	import { goto } from '$app/navigation';

	function handleLogout() {
		auth.logout();
		goto('/');
	}

	function handleSettings() {
		goto('/settings/profile');
	}

	function getInitials(name?: string, email?: string): string {
		if (name) {
			return name
				.split(' ')
				.map((n) => n[0])
				.join('')
				.toUpperCase()
				.slice(0, 2);
		}
		if (email) {
			return email[0].toUpperCase();
		}
		return 'U';
	}
</script>

{#if $currentUser}
	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			<Button variant="ghost" class="relative h-8 w-8 rounded-full p-0">
				<Avatar.Root class="h-8 w-8">
					<Avatar.Image
						src={$currentUser.avatar}
						alt={$currentUser.name || $currentUser.email}
					/>
					<Avatar.Fallback
						>{getInitials($currentUser.name, $currentUser.email)}</Avatar.Fallback
					>
				</Avatar.Root>
			</Button>
		</DropdownMenu.Trigger>
		<DropdownMenu.Content class="w-56" align="end">
			<DropdownMenu.Label class="font-normal">
				<div class="flex flex-col space-y-1">
					<p class="text-sm font-medium leading-none">
						{$currentUser.name || 'User'}
					</p>
					<p class="text-xs leading-none text-muted-foreground">
						{$currentUser.email}
					</p>
				</div>
			</DropdownMenu.Label>
			<DropdownMenu.Separator />
			<DropdownMenu.Group>
				<DropdownMenu.Item onclick={handleSettings}>
					<UserIcon class="mr-2 h-4 w-4" />
					<span>Profile Settings</span>
				</DropdownMenu.Item>
			</DropdownMenu.Group>
			<DropdownMenu.Separator />
			<DropdownMenu.Item onclick={handleLogout}>
				<LogOutIcon class="mr-2 h-4 w-4" />
				<span>Log out</span>
			</DropdownMenu.Item>
		</DropdownMenu.Content>
	</DropdownMenu.Root>
{/if}
