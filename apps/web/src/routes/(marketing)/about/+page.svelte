<script lang="ts">
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Button } from '@eurora/ui/components/button/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import { SiBluesky, SiDiscord, SiGithub, SiReddit, SiX } from '@icons-pack/svelte-simple-icons';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import GithubIcon from '@lucide/svelte/icons/github';
	import LinkedinIcon from '@lucide/svelte/icons/linkedin';
	import MailIcon from '@lucide/svelte/icons/mail';

	const email = 'contact@eurora-labs.com';
	let copied = $state(false);
	let copiedLaura = $state(false);
	let copiedAndre = $state(false);

	async function copyEmail() {
		await navigator.clipboard.writeText(email);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}

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

	const socials = [
		{ name: 'GitHub', href: 'https://github.com/eurora-labs/eurora', icon: SiGithub },
		{ name: 'Discord', href: 'https://discord.gg/xRT9EpBEwc', icon: SiDiscord },
		{ name: 'Reddit', href: 'https://reddit.com/r/eurora', icon: SiReddit },
		{
			name: 'Bluesky',
			href: 'https://bsky.app/profile/euroralabs.bsky.social',
			icon: SiBluesky,
		},
		{ name: 'X', href: 'https://x.com/euroralabs', icon: SiX },
	];

	const laura_thommen_img = 'https://d26xptavrz5c8t.cloudfront.net/image/laura.png';
	const andre_roelofs_img = 'https://d26xptavrz5c8t.cloudfront.net/image/andre.png';
</script>

<div class="container mx-auto max-w-5xl px-4 pt-16 pb-24">
	<div class="mb-6">
		<p class="text-sm font-medium tracking-widest uppercase text-primary mb-3">Who we are</p>
		<h1 class="text-4xl font-bold mb-4 sm:text-5xl">
			Two people, one belief:<br />European AI should work for you.
		</h1>
		<p class="max-w-2xl text-lg text-muted-foreground leading-relaxed">
			Eurora is a company based in The Netherlands, founded by two people who got tired of AI
			tools that treat your data as the product. We're not backed by venture capital or built
			by a faceless corporation — we decided to build something better, rooted in Europe's
			strong tradition of privacy and user rights.
		</p>
	</div>

	<Separator class="mb-16" />

	<div class="grid grid-cols-1 gap-6 mb-20 md:grid-cols-2">
		<div class="group relative">
			<div
				class="absolute -inset-px rounded-2xl bg-gradient-to-b from-primary/20 to-transparent opacity-0 transition-opacity group-hover:opacity-100"
			></div>
			<div class="relative rounded-2xl border border-border bg-card overflow-hidden">
				<div class="overflow-hidden bg-muted">
					<img
						src={laura_thommen_img}
						alt="Laura Thommen"
						class="w-full transition-transform duration-500 group-hover:scale-[1.02]"
					/>
				</div>
				<div class="p-6 sm:p-8">
					<div class="flex items-center gap-3 mb-3">
						<h2 class="text-2xl font-bold">Laura Thommen</h2>
						<Badge variant="secondary">CEO</Badge>
					</div>
					<p class="text-muted-foreground mb-6 leading-relaxed">
						30+ years driving product adoption at Apple, Adobe, Avid, DDN Storage and
						Northrop Grumman. Laura has seen firsthand how technology succeeds when it
						puts people first — and fails when it doesn't.
					</p>

					<div class="space-y-4">
						<div>
							<h3 class="text-sm font-semibold mb-1">Background</h3>
							<p class="text-sm text-muted-foreground">
								Sales leadership to executive management. Built and scaled
								go-to-market teams across enterprise software and hardware.
							</p>
						</div>
						<div>
							<h3 class="text-sm font-semibold mb-1">Education</h3>
							<p class="text-sm text-muted-foreground">
								MBA, Pepperdine University. BS Business Administration (Finance
								minor), University of Southern California.
							</p>
						</div>
					</div>

					<Separator class="my-5" />

					<div class="flex gap-2">
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
		</div>

		<div class="group relative">
			<div
				class="absolute -inset-px rounded-2xl bg-gradient-to-b from-primary/20 to-transparent opacity-0 transition-opacity group-hover:opacity-100"
			></div>
			<div class="relative rounded-2xl border border-border bg-card overflow-hidden">
				<div class="overflow-hidden bg-muted">
					<img
						src={andre_roelofs_img}
						alt="Andre Roelofs"
						class="w-full transition-transform duration-500 group-hover:scale-[1.02]"
					/>
				</div>
				<div class="p-6 sm:p-8">
					<div class="flex items-center gap-3 mb-3">
						<h2 class="text-2xl font-bold">Andre Roelofs</h2>
						<Badge variant="secondary">CTO</Badge>
					</div>
					<p class="text-muted-foreground mb-6 leading-relaxed">
						Former Head of Engineering & ML at Cuebric, where he led development of
						award-winning AI solutions. Andre founded Eurora because he believed the
						best AI tools shouldn't require giving up your privacy.
					</p>

					<div class="space-y-4">
						<div>
							<h3 class="text-sm font-semibold mb-1">Background</h3>
							<p class="text-sm text-muted-foreground">
								Software architecture, machine learning, and computer vision. Built
								production AI systems from research through deployment.
							</p>
						</div>
						<div>
							<h3 class="text-sm font-semibold mb-1">Education</h3>
							<p class="text-sm text-muted-foreground">
								BS Computer Science, Avans University. Graduate studies in AI,
								Maastricht University.
							</p>
						</div>
					</div>

					<Separator class="my-5" />

					<div class="flex gap-2">
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

	<div class="mb-20">
		<h2 class="text-3xl font-bold mb-3">Why we built this</h2>
		<p class="text-muted-foreground max-w-2xl mb-10 leading-relaxed">
			Every AI assistant we tried had the same trade-off: convenience in exchange for your
			data. We started Eurora to prove that's a false choice. Here's what drives every
			decision we make.
		</p>

		<div class="grid grid-cols-1 gap-px bg-border rounded-2xl overflow-hidden md:grid-cols-2">
			<div class="bg-card p-8">
				<h3 class="font-semibold text-lg mb-2">Your data stays yours</h3>
				<p class="text-sm text-muted-foreground leading-relaxed">
					All data lives in sovereign European infrastructure. We can't read it, we can't
					sell it, and we architected the system so we never could.
				</p>
			</div>
			<div class="bg-card p-8">
				<h3 class="font-semibold text-lg mb-2">Open source, always</h3>
				<p class="text-sm text-muted-foreground leading-relaxed">
					Every line of Eurora's code is public. You don't have to take our word for
					anything — read the source, audit it, run it yourself.
				</p>
			</div>
			<div class="bg-card p-8">
				<h3 class="font-semibold text-lg mb-2">No hidden motives</h3>
				<p class="text-sm text-muted-foreground leading-relaxed">
					We're self-funded. No investors pushing for growth-at-all-costs. Our only
					incentive is to build something you genuinely want to use.
				</p>
			</div>
			<div class="bg-card p-8">
				<h3 class="font-semibold text-lg mb-2">Built for people</h3>
				<p class="text-sm text-muted-foreground leading-relaxed">
					We design for how people actually work — across platforms, across browsers,
					without friction. Technology should adapt to you, never the reverse.
				</p>
			</div>
		</div>
	</div>

	<div class="mb-20">
		<h2 class="text-3xl font-bold mb-3">Where we stand</h2>
		<p class="text-muted-foreground max-w-2xl mb-10 leading-relaxed">
			Eurora is young, self-funded, and growing. We're transparent about where we are because
			trust is built on honesty, not marketing.
		</p>

		<div class="grid grid-cols-2 gap-4 sm:grid-cols-4">
			{#each [{ value: '2025', label: 'Founded' }, { value: '100%', label: 'Open Source' }, { value: 'EU', label: 'Data Residency' }, { value: '$0', label: 'VC Funding' }] as stat}
				<div class="rounded-xl border border-border bg-card p-5 text-center">
					<p class="text-2xl font-bold text-primary">{stat.value}</p>
					<p class="text-sm text-muted-foreground mt-1">{stat.label}</p>
				</div>
			{/each}
		</div>
	</div>

	<div class="rounded-2xl bg-foreground/5 p-8 sm:p-12 text-center">
		<MailIcon class="h-8 w-8 text-primary mx-auto mb-4" />
		<h2 class="text-2xl font-bold mb-3">Want to talk?</h2>
		<p class="text-muted-foreground max-w-lg mx-auto mb-6">
			Whether you have questions, feedback, or just want to say hello — we read every message.
		</p>

		<Button variant="outline" onclick={copyEmail} class="font-mono text-sm mb-6">
			{email}
			<CopyIcon class="h-3.5 w-3.5 text-muted-foreground" />
			{#if copied}
				<span class="text-xs text-primary">Copied!</span>
			{/if}
		</Button>

		<div class="flex items-center justify-center gap-2 flex-wrap">
			{#each socials as social}
				<Button
					variant="outline"
					href={social.href}
					target="_blank"
					rel="noopener noreferrer"
					aria-label={social.name}
					class="size-12"
				>
					<social.icon size={20} />
				</Button>
			{/each}
		</div>
	</div>
</div>
