<script lang="ts">
	import { Calendar, Smile, Calculator, User, CreditCard, Settings } from '@lucide/svelte';
	import { Input } from '@eurora/ui';

	import { Launcher } from '@eurora/launcher';
</script>

<div>
	<div
		class="launcher absolute left-1/2 top-1/4 h-[100px] w-[1100px] -translate-x-1/2 -translate-y-1/4"
	>
		<Launcher.Root class="rounded-lg border shadow-md">
			<Launcher.Input placeholder="Search" />
			<Launcher.List>
				<Launcher.Empty>No results found.</Launcher.Empty>
				<Launcher.Group heading="Suggestions">
					<Launcher.Item>
						<Calendar />
						<span>Calendar</span>
					</Launcher.Item>
					<Launcher.Item>
						<Smile />
						<span>Search Emoji</span>
					</Launcher.Item>
					<Launcher.Item disabled>
						<Calculator />
						<span>Calculator</span>
					</Launcher.Item>
				</Launcher.Group>
				<Launcher.Separator />
				<Launcher.Group heading="Settings">
					<Launcher.Item>
						<User />
						<span>Profile</span>
						<Launcher.Shortcut>⌘P</Launcher.Shortcut>
					</Launcher.Item>
					<Launcher.Item>
						<CreditCard />
						<span>Billing</span>
						<Launcher.Shortcut>⌘B</Launcher.Shortcut>
					</Launcher.Item>
					<Launcher.Item>
						<Settings />
						<span>Settings</span>
						<Launcher.Shortcut>⌘S</Launcher.Shortcut>
					</Launcher.Item>
				</Launcher.Group>
			</Launcher.List>
		</Launcher.Root>
		<!-- <Search /> -->
		<!-- <Input type="text" placeholder="Eurora Search" class="h-full w-full border-none text-[32px]" /> -->
		<!-- <Launcher /> -->
	</div>
	<img class=" w-full" src="/sample_background.jpg" alt="Sample Background" />
</div>

<style>
	.launcher {
		background: rgba(255, 255, 255, 0.2);
		box-shadow: 0 4px 30px rgba(0, 0, 0, 0.1);
		backdrop-filter: blur(18px);
		-webkit-backdrop-filter: blur(18px);
	}
</style>
