<script lang="ts">
	import {
		APPEARANCE_SERVICE,
		MAX_SCALE,
		MIN_SCALE,
		SCALE_STEP,
		type Theme,
	} from '$lib/services/appearance-service.svelte.js';
	import { inject } from '@eurora/shared/context';
	import { Button } from '@eurora/ui/components/button/index';
	import { Label } from '@eurora/ui/components/label/index';
	import * as RadioGroup from '@eurora/ui/components/radio-group/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { Slider } from '@eurora/ui/components/slider/index';
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

	const percentFormatter = new Intl.NumberFormat(undefined, {
		style: 'percent',
		maximumFractionDigits: 0,
	});

	function formatScale(value: number): string {
		return percentFormatter.format(value);
	}

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

	async function onInterfaceScaleCommit(value: number) {
		try {
			await appearance.commitInterfaceScale(value);
		} catch (error) {
			toast.error(`Failed to update interface scale: ${error}`);
		}
	}

	async function onTextScaleCommit(value: number) {
		try {
			await appearance.commitTextScale(value);
		} catch (error) {
			toast.error(`Failed to update text size: ${error}`);
		}
	}

	async function onResetScales() {
		try {
			await appearance.resetScales();
		} catch (error) {
			toast.error(`Failed to reset accessibility scales: ${error}`);
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

	<section class="flex flex-col gap-4">
		<div class="flex items-baseline justify-between gap-4">
			<h2 class="text-sm font-medium text-muted-foreground">Accessibility</h2>
			<Button variant="ghost" size="sm" onclick={onResetScales}>Reset to defaults</Button>
		</div>
		<Separator />

		<div class="flex flex-col gap-3">
			<div class="flex items-baseline justify-between gap-4">
				<span id="interface-scale-label" class="text-sm font-medium">
					Interface scale
				</span>
				<span
					class="text-sm tabular-nums text-muted-foreground"
					aria-live="polite"
					aria-atomic="true"
				>
					{formatScale(appearance.interfaceScale)}
				</span>
			</div>
			<span id="interface-scale-description" class="text-xs text-muted-foreground">
				Scales the entire interface — text, controls, and spacing — together. Useful when
				everything feels too small.
			</span>
			<Slider
				type="single"
				value={appearance.interfaceScale}
				min={MIN_SCALE}
				max={MAX_SCALE}
				step={SCALE_STEP}
				aria-labelledby="interface-scale-label"
				aria-describedby="interface-scale-description"
				aria-valuetext={formatScale(appearance.interfaceScale)}
				onValueChange={(value) => appearance.previewInterfaceScale(value as number)}
				onValueCommit={(value) => onInterfaceScaleCommit(value as number)}
				class="mt-1"
			/>
		</div>

		<div class="flex flex-col gap-3">
			<div class="flex items-baseline justify-between gap-4">
				<span id="text-scale-label" class="text-sm font-medium">Text size</span>
				<span
					class="text-sm tabular-nums text-muted-foreground"
					aria-live="polite"
					aria-atomic="true"
				>
					{formatScale(appearance.textScale)}
				</span>
			</div>
			<span id="text-scale-description" class="text-xs text-muted-foreground">
				Increases text size on top of the interface scale, leaving controls and spacing
				untouched. Useful when the chrome is fine but reading is hard.
			</span>
			<Slider
				type="single"
				value={appearance.textScale}
				min={MIN_SCALE}
				max={MAX_SCALE}
				step={SCALE_STEP}
				aria-labelledby="text-scale-label"
				aria-describedby="text-scale-description"
				aria-valuetext={formatScale(appearance.textScale)}
				onValueChange={(value) => appearance.previewTextScale(value as number)}
				onValueCommit={(value) => onTextScaleCommit(value as number)}
				class="mt-1"
			/>
		</div>
	</section>
</div>
