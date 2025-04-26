<script lang="ts">
	import { Command, Card } from '@eurora/ui';
	import { Calendar, Calculator, Settings, User, CreditCard, Smile } from '@lucide/svelte';
</script>

<Command.Root class="max-w-[450px] rounded-lg border shadow-md h-fit">
	<Command.Input placeholder="Type a command or search..." />
	<Command.List>
		<Command.Empty>No results found.</Command.Empty>
		<Command.Group heading="Suggestions">
			<Command.Item>
				<Calendar />
				<span>Calendar</span>
			</Command.Item>
			<Command.Item>
				<Smile />
				<span>Search Emoji</span>
			</Command.Item>
			<Command.Item disabled>
				<Calculator />
				<span>Calculator</span>
			</Command.Item>
		</Command.Group>
		<Command.Separator />
		<Command.Group heading="Settings">
			<Command.Item>
				<User />
				<span>Profile</span>
				<Command.Shortcut>⌘P</Command.Shortcut>
			</Command.Item>
			<Command.Item>
				<CreditCard />
				<span>Billing</span>
				<Command.Shortcut>⌘B</Command.Shortcut>
			</Command.Item>
			<Command.Item>
				<Settings />
				<span>Settings</span>
				<Command.Shortcut>⌘S</Command.Shortcut>
			</Command.Item>
		</Command.Group>
	</Command.List>
</Command.Root>
