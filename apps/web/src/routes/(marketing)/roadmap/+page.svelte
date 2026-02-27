<script lang="ts">
	import { Badge } from '@eurora/ui/components/badge/index';
	import * as Card from '@eurora/ui/components/card/index';
	import { Progress } from '@eurora/ui/components/progress/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import BrainIcon from '@lucide/svelte/icons/brain';
	import CheckCircleIcon from '@lucide/svelte/icons/check-circle';
	import CircleIcon from '@lucide/svelte/icons/circle';
	import CircleDotIcon from '@lucide/svelte/icons/circle-dot';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import LayersIcon from '@lucide/svelte/icons/layers';
	import RocketIcon from '@lucide/svelte/icons/rocket';
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';

	type Phase = {
		id: string;
		title: string;
		subtitle: string;
		status: 'completed' | 'active' | 'upcoming';
		progress: number;
		quarter: string;
		icon: typeof RocketIcon;
		color: string;
		items: { label: string; done: boolean }[];
	};

	const phases: Phase[] = [
		{
			id: 'foundation',
			title: 'Foundation',
			subtitle: 'Core platform & infrastructure',
			status: 'completed',
			progress: 100,
			quarter: 'Q1 2026',
			icon: RocketIcon,
			color: 'var(--chart-1)',
			items: [
				{ label: 'Desktop app for macOS, Windows & Linux', done: true },
				{
					label: 'Browser extension (Chrome, Firefox, Edge, Safari, Librewolf, Brave and all others)',
					done: true,
				},
				{ label: 'European cloud infrastructure', done: true },
				{ label: 'Conversation history & sync', done: true },
			],
		},
		{
			id: 'integration',
			title: 'Full integration',
			subtitle: 'Support for real time context from all desktop apps',
			status: 'active',
			progress: 20,
			quarter: 'Q2 2026',
			icon: BrainIcon,
			color: 'var(--chart-2)',
			items: [
				{ label: 'Stable platform for extension generation', done: true },
				{ label: 'Custom integrations with 100 most popular websites', done: false },
				{ label: 'Chat text and semantic search', done: false },
				{ label: 'Integrations with all desktop apps via standard strategy', done: false },
				{
					label: 'Ability to access your chats from messaging clients like WhatsApp, Telegram, Signal, Discord, and more',
					done: false,
				},
				{ label: 'Multi-model provider support', done: false },
			],
		},
		{
			id: 'memories',
			title: 'Memories',
			subtitle: 'Full support for recording, storing and managing memories',
			status: 'upcoming',
			progress: 0,
			quarter: 'Q3 2026',
			icon: LayersIcon,
			color: 'var(--chart-3)',
			items: [
				{ label: 'Interactive timeline', done: false },
				{
					label: 'Ability to go back in time and ask questions as if no time has passed',
					done: false,
				},
			],
		},
		{
			id: 'file_system',
			title: 'Local and cloud file system integrations',
			subtitle: 'Instantly access and manage files from anywhere',
			status: 'upcoming',
			progress: 0,
			quarter: 'Q4 2026',
			icon: SparklesIcon,
			color: 'var(--chart-4)',
			items: [
				{ label: 'Local file system integration', done: false },
				{ label: 'Support for all the cloud file systems', done: false },
			],
		},
	];

	function statusBadgeVariant(status: Phase['status']) {
		if (status === 'completed') return 'default' as const;
		if (status === 'active') return 'secondary' as const;
		return 'outline' as const;
	}

	function statusLabel(status: Phase['status']) {
		if (status === 'completed') return 'Shipped';
		if (status === 'active') return 'In Progress';
		return 'Planned';
	}
</script>

<div class="container mx-auto max-w-5xl px-4 pt-16 pb-24">
	<div class="mb-6">
		<p class="text-sm font-medium tracking-widest uppercase text-primary mb-3">
			Where we're headed
		</p>
		<h1 class="text-4xl font-bold mb-4 sm:text-5xl">Roadmap</h1>
		<p class="max-w-2xl text-lg text-muted-foreground leading-relaxed">
			Eurora is built in the open. Here's what we've shipped, what we're working on, and where
			we're going next. Priorities shift based on what our users need most.
		</p>
	</div>

	<Separator class="mb-16" />

	<div class="grid grid-cols-2 gap-4 mb-16 sm:grid-cols-4">
		{#each [{ icon: RocketIcon, value: '3', label: 'Platforms' }, { icon: ShieldIcon, value: 'EU', label: 'Data Residency' }, { icon: GlobeIcon, value: '5+', label: 'Browsers' }, { icon: BrainIcon, value: 'Multi', label: 'Model Support' }] as stat}
			<div
				class="group relative rounded-xl border border-border bg-card p-5 transition-colors hover:border-primary/30"
			>
				<stat.icon
					class="h-5 w-5 text-primary mb-3 transition-transform group-hover:scale-110"
				/>
				<p class="text-2xl font-bold">{stat.value}</p>
				<p class="text-sm text-muted-foreground">{stat.label}</p>
			</div>
		{/each}
	</div>

	<div class="relative">
		<div class="absolute left-[19px] top-0 bottom-0 w-px bg-border sm:left-[23px]"></div>

		<div class="flex flex-col gap-12">
			{#each phases as phase, _i}
				<div
					class="relative grid grid-cols-[40px_1fr] gap-4 sm:grid-cols-[48px_1fr] sm:gap-6"
				>
					<div class="relative flex flex-col items-center">
						<div
							class="relative z-10 flex h-10 w-10 items-center justify-center rounded-full border-2 sm:h-12 sm:w-12"
							style="border-color: {phase.color}; background-color: color-mix(in oklch, {phase.color} 12%, transparent);"
						>
							{#if phase.status === 'completed'}
								<CheckCircleIcon
									class="h-5 w-5 sm:h-6 sm:w-6"
									style="color: {phase.color};"
								/>
							{:else if phase.status === 'active'}
								<CircleDotIcon
									class="h-5 w-5 sm:h-6 sm:w-6"
									style="color: {phase.color};"
								/>
							{:else}
								<CircleIcon
									class="h-5 w-5 sm:h-6 sm:w-6"
									style="color: {phase.color};"
								/>
							{/if}
						</div>
					</div>

					<div class="pb-2">
						<div class="flex flex-wrap items-center gap-3 mb-1">
							<span
								class="text-xs font-semibold tracking-wider uppercase text-muted-foreground"
								>{phase.quarter}</span
							>
							<Badge variant={statusBadgeVariant(phase.status)}
								>{statusLabel(phase.status)}</Badge
							>
						</div>

						<h2 class="text-2xl font-bold mb-1">{phase.title}</h2>
						<p class="text-muted-foreground mb-5">{phase.subtitle}</p>

						{#if phase.status !== 'upcoming'}
							<div class="mb-5 max-w-sm">
								<div
									class="flex justify-between text-xs text-muted-foreground mb-1.5"
								>
									<span
										>{phase.items.filter((t) => t.done).length} of {phase.items
											.length}</span
									>
									<span>{phase.progress}%</span>
								</div>
								<Progress value={phase.progress} />
							</div>
						{/if}

						<Card.Root class="border-border/60 bg-card/50 backdrop-blur-xs p-0">
							<Card.Content class="p-0">
								<ul class="divide-y divide-border/50">
									{#each phase.items as item}
										<li
											class="flex items-center gap-3 px-5 py-3.5 text-sm transition-colors hover:bg-muted/50"
										>
											{#if item.done}
												<CheckCircleIcon
													class="h-4 w-4 shrink-0 text-primary"
												/>
												<span>{item.label}</span>
											{:else}
												<CircleIcon
													class="h-4 w-4 shrink-0 text-muted-foreground/50"
												/>
												<span class="text-muted-foreground"
													>{item.label}</span
												>
											{/if}
										</li>
									{/each}
								</ul>
							</Card.Content>
						</Card.Root>
					</div>
				</div>
			{/each}
		</div>
	</div>

	<Separator class="my-16" />

	<div class="rounded-2xl bg-foreground/5 p-8 sm:p-12 text-center">
		<h2 class="text-2xl font-bold mb-3">Something missing?</h2>
		<p class="text-muted-foreground max-w-lg mx-auto mb-6">
			We shape our roadmap around what matters to you. If there's a feature you'd love to see,
			let us know — or open an issue on GitHub.
		</p>
		<div class="flex items-center justify-center gap-3 flex-wrap">
			<a
				href="https://github.com/eurora-labs/eurora/issues"
				target="_blank"
				rel="noopener noreferrer"
				class="inline-flex items-center gap-2 rounded-full bg-primary px-5 py-2.5 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
			>
				Open an Issue
			</a>
			<a
				href="/contact"
				class="inline-flex items-center gap-2 rounded-full border border-border px-5 py-2.5 text-sm font-medium transition-colors hover:bg-accent hover:text-accent-foreground"
			>
				Contact Us
			</a>
		</div>
	</div>
</div>
