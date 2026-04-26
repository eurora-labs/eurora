<script lang="ts">
	import * as Accordion from '@eurora/ui/components/accordion/index';
	import { Avatar, AvatarFallback, AvatarImage } from '@eurora/ui/components/avatar/index';
	import { Button } from '@eurora/ui/components/button/index';
	import CopyIcon from '@lucide/svelte/icons/copy';
	import GitBranchIcon from '@lucide/svelte/icons/git-branch';
	import MegaphoneIcon from '@lucide/svelte/icons/megaphone';
	import PlugIcon from '@lucide/svelte/icons/plug';
	import QuoteIcon from '@lucide/svelte/icons/quote';
	import ShieldIcon from '@lucide/svelte/icons/shield';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import UsersIcon from '@lucide/svelte/icons/users';
	import WrenchIcon from '@lucide/svelte/icons/wrench';

	const email = 'partners@eurora-labs.com';
	const andreImg = 'https://d26xptavrz5c8t.cloudfront.net/image/andre.png';
	let copied = $state(false);

	async function copyEmail() {
		await navigator.clipboard.writeText(email);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<svelte:head>
	<title>Partners - Eurora Labs</title>
	<meta
		name="description"
		content="Join the Eurora partner program. Build on privacy-first, open-source AI with EU data residency. Access technical enablement, co-marketing, and early product influence."
	/>
</svelte:head>

<div class="mb-16">
	<p class="typo-body font-medium tracking-widest uppercase text-primary mb-3">Partner Program</p>
	<h1 class="typo-title mb-4">Interested in becoming an Eurora partner?</h1>
	<p class="typo-body max-w-2xl text-muted-foreground mb-4">
		Eurora partners with forward-thinking companies to build the future of private, open-source
		AI. Our partner program is designed for shared growth, offering access to our platform,
		developer ecosystem, and collaborative enablement programs, along with hands-on support to
		help partners differentiate, scale, and succeed faster.
	</p>
	<p class="typo-body text-muted-foreground">
		Open Source &middot; EU Data Residency &middot; Self-Hostable
	</p>
</div>

<div class="mb-16">
	<h2 class="typo-title mb-4">Who are our partners</h2>
	<p class="typo-body text-muted-foreground max-w-2xl mb-10">
		Whether you're integrating with Eurora, deploying it for clients, or extending it in the
		open, there's a place in the program for you.
	</p>
	<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
		<div class="rounded-2xl bg-card p-8">
			<PlugIcon class="size-5 text-primary mb-3" />
			<h3 class="typo-body font-semibold mb-2">Technology Partners</h3>
			<p class="typo-body text-muted-foreground">
				Model providers, infrastructure platforms, and developer tools building integrations
				with Eurora.
			</p>
		</div>
		<div class="rounded-2xl bg-card p-8">
			<UsersIcon class="size-5 text-primary mb-3" />
			<h3 class="typo-body font-semibold mb-2">Solution Partners</h3>
			<p class="typo-body text-muted-foreground">
				Agencies, consultancies, and system integrators deploying Eurora for end customers.
			</p>
		</div>
		<div class="rounded-2xl bg-card p-8">
			<GitBranchIcon class="size-5 text-primary mb-3" />
			<h3 class="typo-body font-semibold mb-2">Open Source Contributors</h3>
			<p class="typo-body text-muted-foreground">
				Community projects extending Eurora or building on top of our open-source stack.
			</p>
		</div>
	</div>
</div>

<div class="mb-16">
	<h2 class="typo-title mb-4">What partners can expect</h2>
	<p class="typo-body text-muted-foreground max-w-2xl mb-10">
		Hands-on support and shared upside, designed around how you actually work with us.
	</p>
	<div class="grid grid-cols-1 gap-4 md:grid-cols-2">
		<div class="rounded-2xl bg-card p-8">
			<WrenchIcon class="size-5 text-primary mb-3" />
			<h3 class="typo-body font-semibold mb-2">Technical Enablement &amp; Support</h3>
			<p class="typo-body text-muted-foreground">
				Training, integration credits, and direct access to Eurora engineers to help your
				team build, deploy, and sell with confidence.
			</p>
		</div>
		<div class="rounded-2xl bg-card p-8">
			<MegaphoneIcon class="size-5 text-primary mb-3" />
			<h3 class="typo-body font-semibold mb-2">Marketing &amp; Brand Amplification</h3>
			<p class="typo-body text-muted-foreground">
				Co-marketing opportunities, joint content, events, and visibility that elevate your
				brand and showcase shared success.
			</p>
		</div>
		<div class="rounded-2xl bg-card p-8">
			<SparklesIcon class="size-5 text-primary mb-3" />
			<h3 class="typo-body font-semibold mb-2">Product Access &amp; Influence</h3>
			<p class="typo-body text-muted-foreground">
				Early access to new features, roadmap visibility, and the opportunity to shape what
				comes next.
			</p>
		</div>
		<div class="rounded-2xl bg-card p-8">
			<ShieldIcon class="size-5 text-primary mb-3" />
			<h3 class="typo-body font-semibold mb-2">Privacy-First Positioning</h3>
			<p class="typo-body text-muted-foreground">
				Differentiate with a partner that guarantees EU data residency, open-source
				transparency, and zero data exploitation.
			</p>
		</div>
	</div>
</div>

<div class="mb-16">
	<h2 class="typo-title mb-4">How it works</h2>
	<p class="typo-body text-muted-foreground max-w-2xl mb-10">
		A short, predictable path from first contact to going to market together.
	</p>
	<div class="grid grid-cols-1 gap-4 sm:grid-cols-2 md:grid-cols-4">
		<div class="rounded-2xl bg-card p-8">
			<p class="typo-body text-xs font-medium tracking-widest uppercase text-primary mb-2">
				Step 1
			</p>
			<h3 class="typo-body font-semibold mb-2">Reach out</h3>
			<p class="typo-body text-muted-foreground">
				Tell us about your team and how you'd like to work together.
			</p>
		</div>
		<div class="rounded-2xl bg-card p-8">
			<p class="typo-body text-xs font-medium tracking-widest uppercase text-primary mb-2">
				Step 2
			</p>
			<h3 class="typo-body font-semibold mb-2">Discovery call</h3>
			<p class="typo-body text-muted-foreground">
				A short conversation to align on goals, fit, and shared opportunities.
			</p>
		</div>
		<div class="rounded-2xl bg-card p-8">
			<p class="typo-body text-xs font-medium tracking-widest uppercase text-primary mb-2">
				Step 3
			</p>
			<h3 class="typo-body font-semibold mb-2">Technical alignment</h3>
			<p class="typo-body text-muted-foreground">
				Hands-on sessions with our engineering team to scope integration and enablement.
			</p>
		</div>
		<div class="rounded-2xl bg-card p-8">
			<p class="typo-body text-xs font-medium tracking-widest uppercase text-primary mb-2">
				Step 4
			</p>
			<h3 class="typo-body font-semibold mb-2">Joint launch</h3>
			<p class="typo-body text-muted-foreground">
				Go to market together with co-marketing, support, and a shared roadmap.
			</p>
		</div>
	</div>
</div>

<div class="mb-16 rounded-2xl bg-card p-8 sm:p-12">
	<QuoteIcon class="size-8 text-primary mb-4" />
	<blockquote class="typo-body max-w-3xl mb-6">
		At Eurora Labs, we believe in the power of open source to drive innovation and empower
		people to succeed. We're committed to bring European Open Source AI to the world.
	</blockquote>
	<div class="flex items-center gap-3">
		<Avatar class="size-12 ring-2 ring-border">
			<AvatarImage src={andreImg} alt="Andre Roelofs" />
			<AvatarFallback>AR</AvatarFallback>
		</Avatar>
		<div>
			<p class="typo-body font-semibold">Andre Roelofs</p>
			<p class="typo-body text-muted-foreground">CTO &amp; Co-Founder, Eurora Labs</p>
		</div>
	</div>
</div>

<div class="mb-16 grid grid-cols-1 gap-12 md:grid-cols-3">
	<div class="md:col-span-1">
		<h2 class="typo-title mb-4">Built for teams in</h2>
		<p class="typo-body text-muted-foreground">
			Industries where privacy, sovereignty, and auditability aren't optional.
		</p>
	</div>
	<div class="md:col-span-2">
		<div class="grid grid-cols-2 gap-3 sm:grid-cols-3">
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">Media &amp; Entertainment</p>
			</div>
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">High Performance Computing</p>
			</div>
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">Academia</p>
			</div>
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">Healthcare</p>
			</div>
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">Financial Services</p>
			</div>
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">Government</p>
			</div>
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">Life Sciences</p>
			</div>
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">Automotive</p>
			</div>
			<div class="rounded-xl bg-card px-5 py-4">
				<p class="typo-body font-semibold">Manufacturing</p>
			</div>
		</div>
	</div>
</div>

<div class="mb-16 grid grid-cols-1 gap-12 md:grid-cols-3">
	<div class="md:col-span-1">
		<h2 class="typo-title mb-4">Frequently asked questions</h2>
		<p class="typo-body text-muted-foreground">
			Can't find what you're looking for? Email <a
				href="mailto:partners@eurora-labs.com"
				class="text-primary underline-offset-4 hover:underline">partners@eurora-labs.com</a
			>.
		</p>
	</div>
	<div class="md:col-span-2">
		<Accordion.Root type="single">
			<Accordion.Item value="exclusive">
				<Accordion.Trigger class="text-lg py-5"
					>Do you offer exclusive partnerships?</Accordion.Trigger
				>
				<Accordion.Content class="text-lg leading-relaxed text-muted-foreground">
					By default, partnerships are non-exclusive. We're open to discussing exclusive
					arrangements per region or vertical on a case-by-case basis.
				</Accordion.Content>
			</Accordion.Item>
			<Accordion.Item value="support">
				<Accordion.Trigger class="text-lg py-5"
					>What support do partners receive?</Accordion.Trigger
				>
				<Accordion.Content class="text-lg leading-relaxed text-muted-foreground">
					Partners get direct access to our engineering team, integration support, and
					co-marketing resources. Specific SLAs are defined as part of onboarding.
				</Accordion.Content>
			</Accordion.Item>
			<Accordion.Item value="selfhost">
				<Accordion.Trigger class="text-lg py-5">Can we self-host Eurora?</Accordion.Trigger>
				<Accordion.Content class="text-lg leading-relaxed text-muted-foreground">
					Yes. Eurora is open source and self-hostable.
				</Accordion.Content>
			</Accordion.Item>
			<Accordion.Item value="data">
				<Accordion.Trigger class="text-lg py-5"
					>Where is partner data stored?</Accordion.Trigger
				>
				<Accordion.Content class="text-lg leading-relaxed text-muted-foreground">
					All Eurora-managed services run with EU data residency by default. Self-hosted
					deployments give you full control over data location.
				</Accordion.Content>
			</Accordion.Item>
			<Accordion.Item value="regions">
				<Accordion.Trigger class="text-lg py-5"
					>Which regions do you operate in?</Accordion.Trigger
				>
				<Accordion.Content class="text-lg leading-relaxed text-muted-foreground">
					Eurora is headquartered in the EU and works with partners globally. Most of our
					active engagements are in Europe and North America.
				</Accordion.Content>
			</Accordion.Item>
		</Accordion.Root>
	</div>
</div>

<div class="mb-8">
	<h2 class="typo-title mb-4">See what's possible with AI that respects your privacy.</h2>
	<p class="typo-body text-muted-foreground max-w-2xl mb-6">
		Get in touch and explore how Eurora can power your solutions.
	</p>
	<div class="flex flex-col sm:flex-row items-start gap-4">
		<Button variant="outline" onclick={copyEmail} class="font-mono">
			{email}
			<CopyIcon class="h-4 w-4 text-muted-foreground" />
			{#if copied}
				<span class="typo-body text-primary">Copied!</span>
			{/if}
		</Button>
		<Button href="/contact">Contact Us</Button>
	</div>
</div>

<br />
