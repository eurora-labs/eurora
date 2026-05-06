<script lang="ts">
	import { type Theme } from '$lib/bindings/bindings.js';
	import { APPEARANCE_SERVICE } from '$lib/services/appearance-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Label } from '@eurora/ui/components/label/index';
	import * as RadioGroup from '@eurora/ui/components/radio-group/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Switch } from '@eurora/ui/components/switch/index';
	import { toast } from 'svelte-sonner';

	const appearance = inject(APPEARANCE_SERVICE);

	const themeOptions: { value: Theme; label: string; description: string }[] = [
		{
			value: 'system',
			label: 'System',
			description: 'Match the operating system setting.',
		},
		{
			value: 'light',
			label: 'Light',
			description: 'Always use the light theme.',
		},
		{
			value: 'dark',
			label: 'Dark',
			description: 'Always use the dark theme.',
		},
	];

	async function onThemeChange(value: string) {
		try {
			await appearance.setTheme(value as Theme);
		} catch (error) {
			toast.error(`Failed to update theme: ${error}`);
		}
	}

	async function onDynamicAccentChange(checked: boolean) {
		try {
			await appearance.setDynamicAccent(checked);
		} catch (error) {
			toast.error(`Failed to update accent preference: ${error}`);
		}
	}
</script>

<div class="flex flex-col gap-8">
	<div>
		<h1 class="text-lg font-semibold">Appearance</h1>
		<p class="text-sm text-muted-foreground">Customize how Eurora looks on this device.</p>
	</div>

	<section class="flex flex-col gap-4">
		<h2 class="text-sm font-medium text-muted-foreground">Theme</h2>
		<Separator />
		<RadioGroup.Root value={appearance.theme} onValueChange={onThemeChange}>
			{#each themeOptions as option (option.value)}
				<div class="flex items-start gap-3">
					<RadioGroup.Item id="theme-{option.value}" value={option.value} class="mt-1" />
					<div class="flex flex-col gap-0.5">
						<Label for="theme-{option.value}" class="text-sm font-medium">
							{option.label}
						</Label>
						<span class="text-xs text-muted-foreground">{option.description}</span>
					</div>
				</div>
			{/each}
		</RadioGroup.Root>
	</section>

	<section class="flex flex-col gap-4">
		<h2 class="text-sm font-medium text-muted-foreground">Accent</h2>
		<Separator />
		<div class="flex items-start justify-between gap-4">
			<div class="flex flex-col gap-0.5">
				<Label for="dynamic-accent" class="text-sm font-medium">
					Use activity icon colors for accents
				</Label>
				<span class="text-xs text-muted-foreground">
					Tints buttons and highlights with the dominant color of the focused activity's
					icon.
				</span>
			</div>
			<Switch
				id="dynamic-accent"
				checked={appearance.dynamicAccent}
				onCheckedChange={onDynamicAccentChange}
			/>
		</div>
	</section>
</div>
