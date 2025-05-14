<script lang="ts">
	import {
		Calendar,
		Smile,
		Calculator,
		User,
		CreditCard,
		Settings,
		Captions,
		Film
	} from '@lucide/svelte';

	// import { LauncherNative as Launcher } from '@eurora/launcher';
	import { Launcher } from '@eurora/launcher';
	import { Badge } from '@eurora/ui';
</script>

<div>
	<div class="launcher absolute left-1/2 top-1/4 w-[1100px] -translate-x-1/2">
		<Launcher.Root class="rounded-lg border shadow-md">
			<Launcher.Input placeholder="Search" />
			<!-- <span class="absolute left-[175px] top-4 ml-2 mt-2 flex items-center gap-2">
				<div class="transcript-badge">transcript</div>
				<div class="transcript-badge">video</div>
			</span> -->
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

<style lang="postcss">
	.launcher {
		background: rgba(255, 255, 255, 0.2);
		box-shadow: 0 4px 30px rgba(0, 0, 0, 0.1);
		backdrop-filter: blur(36px);
		-webkit-backdrop-filter: blur(36px);
	}

	.transcript-badge {
		@apply flex w-fit items-center gap-2 text-[40px] leading-[40px] text-white;
		@apply p-2;
		background: rgba(128, 128, 128, 0.4);
		color: rgba(0, 0, 0, 0.8);
		border-radius: 16px;
	}
</style>
