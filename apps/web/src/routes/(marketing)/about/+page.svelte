<script lang="ts">
	import ReadyToStart from '$lib/components/marketing/ReadyToStart.svelte';
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { SiBluesky, SiDiscord, SiGithub, SiReddit, SiX } from '@icons-pack/svelte-simple-icons';
	import GithubIcon from '@lucide/svelte/icons/github';
	import LinkedinIcon from '@lucide/svelte/icons/linkedin';
	import MailIcon from '@lucide/svelte/icons/mail';

	const email = 'contact@eurora-labs.com';
	let copiedLaura = $state(false);
	let copiedAndre = $state(false);

	async function copyLauraEmail() {
		await navigator.clipboard.writeText('laura@eurora-labs.com');
		copiedLaura = true;
		setTimeout(() => (copiedLaura = false), 2000);
	}

	async function copyAndreEmail() {
		await navigator.clipboard.writeText('andre@eurora-labs.com');
		copiedAndre = true;
		setTimeout(() => (copiedAndre = false), 2000);
	}

	const laura_thommen_img = 'https://d26xptavrz5c8t.cloudfront.net/image/laura.png';
	const andre_roelofs_img = 'https://d26xptavrz5c8t.cloudfront.net/image/andre.png';

	const stats = [
		{ value: '2025', label: 'Founded' },
		{ value: '100%', label: 'Open Source' },
		{ value: 'EU', label: 'Data Residency' },
	];
</script>

<svelte:head>
	<title>About — Eurora Labs</title>
	<meta
		name="description"
		content="Meet the team behind Eurora. A Netherlands-based, self-funded company building private, open-source AI tools with EU data residency and full transparency."
	/>
</svelte:head>

<div class="container mx-auto max-w-5xl px-4 pt-16 pb-16">
	<!-- Hero -->
	<div class="text-center mb-16">
		<p class="text-sm font-medium tracking-widest uppercase text-primary mb-4">Who we are</p>
		<h1 class="text-4xl font-bold mb-6 sm:text-5xl lg:text-6xl leading-tight">
			Two people, one belief:<br />
			<span class="text-primary">European AI should work for you.</span>
		</h1>
		<p class="max-w-2xl mx-auto text-lg text-muted-foreground leading-relaxed">
			Eurora is a company based in The Netherlands, founded by two people who got tired of AI
			tools that treat your data as the product. We decided to build something better, rooted
			in Europe's strong tradition of privacy and user rights.
		</p>
	</div>

	<!-- Founders -->
	<div class="mb-16">
		<div class="flex flex-col gap-6 sm:flex-row sm:gap-8 items-stretch">
			<!-- Laura -->
			<div class="group flex-1 relative">
				<div
					class="absolute -inset-px rounded-2xl bg-linear-to-b from-primary/20 to-transparent opacity-0 transition-opacity group-hover:opacity-100"
				></div>
				<div class="relative rounded-2xl bg-card p-6 sm:p-8 h-full flex flex-col">
					<div class="flex items-center gap-4 mb-5">
						<img
							src={laura_thommen_img}
							alt="Laura Thommen"
							class="size-16 rounded-full object-cover ring-2 ring-border"
						/>
						<div>
							<div class="flex items-center gap-2">
								<h2 class="text-xl font-bold">Laura Thommen</h2>
								<Badge variant="secondary">CEO</Badge>
							</div>
							<p class="text-sm text-muted-foreground mt-0.5">
								30+ years in Executive Leadership
							</p>
						</div>
					</div>

					<p class="text-muted-foreground leading-relaxed mb-5">
						Drove product adoption at Apple, Adobe, Avid, DDN Storage and Northrop
						Grumman. Laura has seen firsthand how technology succeeds when it puts
						people first — and fails when it doesn't.
					</p>

					<div class="space-y-3 mb-5 text-sm">
						<div>
							<p class="font-medium mb-1">Background</p>
							<p class="text-muted-foreground">
								Sales leadership to executive management. Built go-to-market teams
								across enterprise software and hardware.
							</p>
						</div>
						<div>
							<p class="font-medium mb-1">Education</p>
							<p class="text-muted-foreground">
								MBA, Pepperdine University. BS Business Administration, USC.
							</p>
						</div>
					</div>

					<div class="flex gap-1 pt-4 border-t border-border">
						<div class="relative">
							{#if copiedLaura}
								<span
									class="absolute -top-8 left-1/2 -translate-x-1/2 rounded-md bg-primary px-2 py-1 text-xs text-primary-foreground whitespace-nowrap"
									>Copied!</span
								>
							{/if}
							<Button
								variant="ghost"
								size="icon"
								class="h-8 w-8"
								onclick={copyLauraEmail}
							>
								<MailIcon class="h-4 w-4" />
							</Button>
						</div>
						<Button
							variant="ghost"
							size="icon"
							class="h-8 w-8"
							href="https://www.linkedin.com/in/laura-thommen-6a40b53/"
							target="_blank"
							rel="noopener noreferrer"
						>
							<LinkedinIcon class="h-4 w-4" />
						</Button>
					</div>
				</div>
			</div>

			<!-- Andre -->
			<div class="group flex-1 relative">
				<div
					class="absolute -inset-px rounded-2xl bg-linear-to-b from-primary/20 to-transparent opacity-0 transition-opacity group-hover:opacity-100"
				></div>
				<div class="relative rounded-2xl bg-card p-6 sm:p-8 h-full flex flex-col">
					<div class="flex items-center gap-4 mb-5">
						<img
							src={andre_roelofs_img}
							alt="Andre Roelofs"
							class="size-16 rounded-full object-cover ring-2 ring-border"
						/>
						<div>
							<div class="flex items-center gap-2">
								<h2 class="text-xl font-bold">Andre Roelofs</h2>
								<Badge variant="secondary">CTO</Badge>
							</div>
							<p class="text-sm text-muted-foreground mt-0.5">
								AI & engineering leadership
							</p>
						</div>
					</div>

					<p class="text-muted-foreground leading-relaxed mb-5">
						Former Head of Engineering &amp; ML at Cuebric, where he led development of
						award-winning AI solutions. Andre founded Eurora because he believed the
						best AI tools shouldn't require giving up your privacy.
					</p>

					<div class="space-y-3 mb-5 text-sm">
						<div>
							<p class="font-medium mb-1">Background</p>
							<p class="text-muted-foreground">
								Software architecture, machine learning, and computer vision.
								Production AI systems from research through deployment.
							</p>
						</div>
						<div>
							<p class="font-medium mb-1">Education</p>
							<p class="text-muted-foreground">
								BS Computer Science, Avans University. Graduate studies in AI,
								Maastricht University.
							</p>
						</div>
					</div>

					<div class="flex gap-1 pt-4 border-t border-border">
						<div class="relative">
							{#if copiedAndre}
								<span
									class="absolute -top-8 left-1/2 -translate-x-1/2 rounded-md bg-primary px-2 py-1 text-xs text-primary-foreground whitespace-nowrap"
									>Copied!</span
								>
							{/if}
							<Button
								variant="ghost"
								size="icon"
								class="h-8 w-8"
								onclick={copyAndreEmail}
							>
								<MailIcon class="h-4 w-4" />
							</Button>
						</div>
						<Button
							variant="ghost"
							size="icon"
							class="h-8 w-8"
							href="https://www.linkedin.com/in/andre-roelofs/"
							target="_blank"
							rel="noopener noreferrer"
						>
							<LinkedinIcon class="h-4 w-4" />
						</Button>
						<Button
							variant="ghost"
							size="icon"
							class="h-8 w-8"
							href="https://github.com/AndreRoelofs"
							target="_blank"
							rel="noopener noreferrer"
						>
							<GithubIcon class="h-4 w-4" />
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>

	<div class="mb-16">
		<h2 class="text-3xl font-bold mb-3">Why we built this</h2>
		<p class="text-muted-foreground max-w-2xl mb-10 leading-relaxed">
			Every AI assistant we tried had the same trade-off: convenience in exchange for your
			data. Eurora Labs has proven that you do not need to compromise convenience in exchange
			for your data. Here's what drives every decision we make.
		</p>

		<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
			<div class="rounded-2xl bg-card p-8">
				<h3 class="font-semibold text-lg mb-2">Your data stays yours</h3>
				<p class="text-sm text-muted-foreground leading-relaxed">
					All data lives in sovereign European infrastructure. We can't read it, we can't
					sell it, and we architected the system so we never could.
				</p>
			</div>
			<div class="rounded-2xl bg-card p-8">
				<h3 class="font-semibold text-lg mb-2">Open source, always</h3>
				<p class="text-sm text-muted-foreground leading-relaxed">
					Every line of Eurora's code is public. You don't have to take our word for
					anything — read the source, audit it, run it yourself.
				</p>
			</div>
			<div class="rounded-2xl bg-card p-8">
				<h3 class="font-semibold text-lg mb-2">No hidden motives</h3>
				<p class="text-sm text-muted-foreground leading-relaxed">
					We're self-funded. No investors pushing for growth-at-all-costs. Our only
					incentive is to build something you genuinely want to use.
				</p>
			</div>
			<div class="rounded-2xl bg-card p-8">
				<h3 class="font-semibold text-lg mb-2">Built for people</h3>
				<p class="text-sm text-muted-foreground leading-relaxed">
					We design for how people actually work — across platforms, across browsers,
					without friction. Technology should adapt to you, never the reverse.
				</p>
			</div>
		</div>
	</div>

	<div class="mb-16">
		<div class="grid grid-cols-2 gap-4 sm:grid-cols-3">
			{#each stats as stat}
				<div class="rounded-2xl bg-card p-6 text-center">
					<p class="text-3xl font-bold text-primary mb-1">{stat.value}</p>
					<p class="text-sm text-muted-foreground">{stat.label}</p>
				</div>
			{/each}
		</div>
	</div>

	<ReadyToStart />
</div>
