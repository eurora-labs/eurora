<script lang="ts">
	import { Avatar, AvatarFallback, AvatarImage } from '@eurora/ui/components/avatar/index';
	import { Button } from '@eurora/ui/components/button/index';
	import GithubIcon from '@lucide/svelte/icons/github';
	import LinkedinIcon from '@lucide/svelte/icons/linkedin';
	import MailIcon from '@lucide/svelte/icons/mail';

	const andre_roelofs_img = 'https://d26xptavrz5c8t.cloudfront.net/image/andre.png';

	let copied = $state(false);

	async function copyEmail() {
		await navigator.clipboard.writeText('andre@eurora-labs.com');
		copied = true;
		setTimeout(() => (copied = false), 2000);
	}
</script>

<svelte:head>
	<title>Why Eurora | Eurora Labs</title>
	<meta
		name="description"
		content="Why Eurora exists: an AI assistant with full context awareness that doesn't sacrifice your privacy. Open source, EU-based, self-hostable, no vendor lock-in."
	/>
</svelte:head>

<div class="mb-4">
	<p class="typo-body font-medium tracking-widest uppercase text-primary mb-4">Our story</p>
	<p class="typo-body max-w-2xl text-muted-foreground">
		There are dozens of AI assistants already. None of them satisfy my needs.
	</p>
</div>

<div class="flex flex-col gap-12 mb-12">
	<section class="space-y-4">
		<h2 class="typo-title mb-2">Challenge</h2>
		<p class="typo-body text-muted-foreground">
			All AI assistants start conversations from scratch. They do remember what you asked in
			the past, but they do not know what you are doing at the moment. If you ask an AI
			"What's this?", you first need to explain what "this" is.
		</p>

		<p class="typo-body text-muted-foreground">
			Asking a question about anything you do becomes an annoying chore instead of an
			opportunity to learn more. If you have a question about a video on YouTube, for example,
			you need to:
		</p>

		<ol class="typo-body list-decimal ml-6 text-muted-foreground space-y-1">
			<li>Scroll down to find the "Show Transcript" button</li>
			<li>Manually highlight the whole transcript</li>
			<li>Navigate to a separate AI assistant</li>
			<li>Paste the transcript</li>
			<li>Explain what the context of this transcript is</li>
		</ol>

		<p class="typo-body text-muted-foreground">
			Finally, you can start asking your question. Hopefully you want something simple like
			"Summarize this". If you instead want to ask a real question about the current sentence
			in the video, you need to be prepared to do even more work to explain yourself. And if
			your question is about the visual content, you also have to screenshot the video and
			paste it into the assistant.
		</p>

		<p class="typo-body text-muted-foreground">
			I quickly found myself dreading this ritual. I wanted my questions answered and my
			curiosity satisfied. AI assistants make for a nice demo but are unsuitable for real
			work. I wanted something real. Something that is fun to use. I wanted an AI assistant
			that could "Explain this".
		</p>
	</section>

	<section class="space-y-4">
		<h2 class="typo-title mb-2">Vision</h2>
		<p class="typo-body text-muted-foreground">
			Eurora is an AI assistant that sees what you see. It understands the context of what you
			are doing at any given moment. The page you are reading, the video you are watching, the
			document you are working on. Instead of copying, pasting, and explaining, you simply ask
			your question. "What does this mean?" "Summarize this." "Explain this." Eurora already
			knows what "this" is.
		</p>
	</section>

	<section class="space-y-4">
		<h2 class="typo-title mb-2">Privacy</h2>
		<p class="typo-body text-muted-foreground">
			An AI assistant that sees what you see raises an obvious question: who else gets to see
			it? To date, every AI assistant that passively observes what you do has failed to gather
			any meaningful number of users. And every single one has received significant backlash.
			Rightfully so.
		</p>
		<p class="typo-body text-muted-foreground">
			I needed to make an app that is private and trustworthy. Something I would use myself.
			The first step was to base the company in The Netherlands, subject to European privacy
			and data protection laws. The second step was to open-source every single line of code.
			Transparency is the key to building trust. Ongoing support for fully local deployments
			also means that users can be confident the app will not go back on its promises.
		</p>
		<p class="typo-body text-muted-foreground">
			Everything about Eurora is driven by the vision of what I would want to use. What anyone
			would want to use.
		</p>
	</section>

	<section class="space-y-4">
		<h2 class="typo-title mb-2">Future</h2>
		<p class="typo-body text-muted-foreground">
			I started building Eurora in January 2025. It took over a year of constant effort and
			countless iterations to get the app right. It only succeeded because of invaluable
			feedback from users like you. I could not have gotten where I am today without your
			help. For that, I am extremely grateful.
		</p>
		<p class="typo-body text-muted-foreground">
			I have shown that AI assistants can be enjoyable to use without compromising on privacy
			and security. Eurora is how I want to use AI going forward. Eurora is ultimately made
			for you, so I would love to hear your thoughts. Please find the social media links
			below. Leave a comment anywhere you want. I read them all and will reply as soon as I
			can.
		</p>
	</section>
</div>

<div class="mb-12 flex justify-end">
	<div>
		<p class="typo-body font-medium tracking-widest uppercase text-primary text-xs mb-2">
			Written by
		</p>
		<div class="flex items-start gap-4 mr-5">
			<Avatar class="size-12 ring-2 ring-border mt-0.5">
				<AvatarImage src={andre_roelofs_img} alt="Andre Roelofs" />
				<AvatarFallback>AR</AvatarFallback>
			</Avatar>
			<div>
				<p class="typo-body font-semibold">Andre Roelofs</p>
				<p class="typo-body text-muted-foreground">CTO &amp; Co-founder</p>
				<div class="flex items-center gap-1 mt-1 -ml-2">
					<Button
						variant="ghost"
						size="icon"
						class="h-7 w-7 text-muted-foreground"
						href="https://www.linkedin.com/in/andre-roelofs/"
						target="_blank"
						rel="noopener noreferrer"
						aria-label="Andre Roelofs on LinkedIn"
					>
						<LinkedinIcon class="h-3.5 w-3.5" />
					</Button>
					<Button
						variant="ghost"
						size="icon"
						class="h-7 w-7 text-muted-foreground"
						href="https://github.com/AndreRoelofs"
						target="_blank"
						rel="noopener noreferrer"
						aria-label="Andre Roelofs on GitHub"
					>
						<GithubIcon class="h-3.5 w-3.5" />
					</Button>
					<div class="relative">
						{#if copied}
							<span
								class="absolute -top-8 left-1/2 -translate-x-1/2 rounded-md bg-primary px-2 py-1 typo-body text-primary-foreground whitespace-nowrap"
								>Copied!</span
							>
						{/if}
						<Button
							variant="ghost"
							size="icon"
							class="h-7 w-7 text-muted-foreground"
							onclick={copyEmail}
							aria-label="Copy email address"
						>
							<MailIcon class="h-3.5 w-3.5" />
						</Button>
					</div>
				</div>
			</div>
		</div>
	</div>
</div>
