<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { Separator } from '@eurora/ui/components/separator/index';
	import CopyIcon from '@lucide/svelte/icons/copy';

	const email = 'partners@eurora-labs.com';
	let copied = $state(false);

	async function copyEmail() {
		await navigator.clipboard.writeText(email);
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<svelte:head>
	<title>Partners — Eurora Labs</title>
	<meta
		name="description"
		content="Join the Eurora partner program. Build on privacy-first, open-source AI with EU data residency. Access preferred pricing, technical enablement, and go-to-market collaboration."
	/>
</svelte:head>

<div class="mb-10">
	<p class="text-sm font-medium tracking-widest uppercase text-primary mb-3">Partner Program</p>
	<h1 class="text-4xl font-bold mb-4 sm:text-5xl">Interested in becoming a Eurora partner?</h1>
	<p class="max-w-3xl text-lg text-muted-foreground leading-relaxed">
		Eurora partners with forward-thinking companies to build the future of private, open-source
		AI. Our partner program is designed for shared growth, offering access to our platform,
		developer ecosystem, and collaborative enablement programs along with meaningful commercial
		upside, hands-on support, and deep go-to-market collaboration to help partners
		differentiate, scale, and succeed faster.
	</p>
</div>

<div class="mb-10">
	<h2 class="text-2xl font-bold mb-6">What partners can expect:</h2>

	<div class="space-y-6 max-w-3xl">
		<div>
			<h3 class="font-bold text-lg mb-2">Commercial Advantages</h3>
			<p class="text-muted-foreground leading-relaxed">
				Preferred pricing, referral incentives, and revenue-sharing programs designed to
				reward partner investment and performance.
			</p>
		</div>

		<div>
			<h3 class="font-bold text-lg mb-2">Technical Enablement & Support</h3>
			<p class="text-muted-foreground leading-relaxed">
				Training, integration credits, and direct access to Eurora engineers to help your
				team build, deploy, and sell with confidence.
			</p>
		</div>

		<div>
			<h3 class="font-bold text-lg mb-2">Marketing & Brand Amplification</h3>
			<p class="text-muted-foreground leading-relaxed">
				Co-marketing opportunities, joint content, events, and visibility that elevate your
				brand and showcase shared success.
			</p>
		</div>

		<div>
			<h3 class="font-bold text-lg mb-2">Go-to-Market Collaboration</h3>
			<p class="text-muted-foreground leading-relaxed">
				Account alignment, sales enablement, and deal support to help you engage the right
				customers and close faster.
			</p>
		</div>

		<div>
			<h3 class="font-bold text-lg mb-2">Product Access & Influence</h3>
			<p class="text-muted-foreground leading-relaxed">
				Early access to new features, roadmap visibility, and the opportunity to shape what
				comes next.
			</p>
		</div>

		<div>
			<h3 class="font-bold text-lg mb-2">Privacy-First Positioning</h3>
			<p class="text-muted-foreground leading-relaxed">
				Differentiate with a partner that guarantees EU data residency, open-source
				transparency, and zero data exploitation.
			</p>
		</div>
	</div>
</div>

<div class="max-w-3xl mb-10">
	<p class="text-muted-foreground leading-relaxed">
		Reach out to learn more about our partner program and apply to get started. We're excited to
		work with you.
	</p>
</div>

<Separator class="mb-10" />

<div class="mb-8">
	<h2 class="text-3xl font-bold mb-3 sm:text-4xl">
		See what's possible with AI that respects your privacy.
	</h2>
	<p class="text-lg text-muted-foreground max-w-2xl mb-6">
		Get in touch and explore how Eurora can power your solutions.
	</p>
	<div class="flex flex-col sm:flex-row items-start gap-4">
		<Button variant="outline" onclick={copyEmail} class="font-mono text-sm">
			{email}
			<CopyIcon class="h-3.5 w-3.5 text-muted-foreground" />
			{#if copied}
				<span class="text-xs text-primary">Copied!</span>
			{/if}
		</Button>
		<Button href="/contact">Contact Us</Button>
	</div>
</div>

<br />
