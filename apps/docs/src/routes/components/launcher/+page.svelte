<script lang="ts">
	import { Calendar, Smile, Calculator, User, CreditCard, Settings, Plus } from '@lucide/svelte';

	import { LauncherNative as Launcher } from '@eurora/launcher';
	import { Badge } from '@eurora/ui';
</script>

<div>
	<div class="launcher absolute left-1/2 top-1/4 w-[1100px] -translate-x-1/2">
		<Launcher.Root class="rounded-lg border shadow-md">
			<Launcher.Input placeholder="Search" class="ml-[180px]" />
			<span class="absolute left-[0px] top-4 ml-2 mt-2 flex items-center gap-2">
				<Badge class="h-[50px] w-fit"><Calendar /> Test 1</Badge>
				<Badge class="h-[50px] w-fit"><CreditCard /> Test 2</Badge>
				<Badge class="h-[50px] w-fit bg-black"><Plus /></Badge>
			</span>
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
					<Launcher.Item>
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
