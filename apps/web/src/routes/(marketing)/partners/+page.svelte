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
	<title>Partners - Eurora Labs</title>
	<meta
		name="description"
		content="Join the Eurora partner program. Build on privacy-first, open-source AI with EU data residency. Access technical enablement, co-marketing, and early product influence."
	/>
</svelte:head>

<div class="mb-10">
	<p class="typo-body font-medium tracking-widest uppercase text-primary mb-3">Partner Program</p>
	<h1 class="typo-title mb-4">Interested in becoming a Eurora partner?</h1>
	<p class="typo-body max-w-3xl text-muted-foreground">
		Eurora partners with forward-thinking companies to build the future of private, open-source
		AI. Our partner program is designed for shared growth, offering access to our platform,
		developer ecosystem, and collaborative enablement programs, along with hands-on support to
		help partners differentiate, scale, and succeed faster.
	</p>
</div>

<div class="mb-10">
	<h2 class="typo-title mb-6">What partners can expect</h2>

	<div class="space-y-6 max-w-3xl">
		<div>
			<h3 class="typo-body font-bold mb-2">Technical Enablement &amp; Support</h3>
			<p class="typo-body text-muted-foreground">
				Training, integration credits, and direct access to Eurora engineers to help your
				team build, deploy, and sell with confidence.
			</p>
		</div>

		<div>
			<h3 class="typo-body font-bold mb-2">Marketing &amp; Brand Amplification</h3>
			<p class="typo-body text-muted-foreground">
				Co-marketing opportunities, joint content, events, and visibility that elevate your
				brand and showcase shared success.
			</p>
		</div>

		<div>
			<h3 class="typo-body font-bold mb-2">Product Access &amp; Influence</h3>
			<p class="typo-body text-muted-foreground">
				Early access to new features, roadmap visibility, and the opportunity to shape what
				comes next.
			</p>
		</div>

		<div>
			<h3 class="typo-body font-bold mb-2">Privacy-First Positioning</h3>
			<p class="typo-body text-muted-foreground">
				Differentiate with a partner that guarantees EU data residency, open-source
				transparency, and zero data exploitation.
			</p>
		</div>
	</div>
</div>

<div class="max-w-3xl mb-10">
	<p class="typo-body text-muted-foreground">
		Reach out to learn more about our partner program and apply to get started. We're excited to
		work with you.
	</p>
</div>

<Separator class="mb-10" />

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
