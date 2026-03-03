<script lang="ts">
	import Zoom from 'svelte-medium-image-zoom';
	import 'svelte-medium-image-zoom/dist/styles.css';
</script>

<div>
	<h1 class="mb-4 text-4xl font-bold">Welcome to Eurora</h1>
	<p class="mb-12 text-lg text-muted-foreground">
		Eurora is a context-aware, cross-platform, open-source AI platform built in Europe. It
		integrates with your operating system and browser to give AI full awareness of what you're
		working on — without selling your data or invading your privacy.
	</p>

	<div class="flex flex-col gap-12">
		<section>
			<h2 class="mb-4 text-2xl font-semibold">Overview</h2>
			<p class="mb-8 text-muted-foreground">
				Eurora is the first AI assistant that provides deep context-awareness across every
				website you visit. Eurora works with every browser across Linux, macOS, and Windows.
				We aim to provide you with the best possible AI assistant experience.
			</p>
			<figure class="mb-6">
				<Zoom>
					<img
						src="https://d26xptavrz5c8t.cloudfront.net/image/linux/youtube_demo.png"
						alt="Eurora desktop app demo showing AI assistant analyzing a YouTube video"
					/>
				</Zoom>

				<figcaption class="mt-2 text-center text-sm text-muted-foreground px-8">
					Eurora's desktop app providing context-aware AI assistance while browsing
					YouTube. In this case, Eurora picked up the current youtube video as an asset.
				</figcaption>
			</figure>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Getting Started</h2>
			<p class="mb-8 text-muted-foreground">
				The easiest way to start using Eurora is to download the app and sign up for a free
				account. Below you will find a video with full overview.
			</p>
		</section>

		<section>
			<h2 class="mb-4 text-2xl font-semibold">Motivation</h2>
			<p class="mb-8 text-muted-foreground">
				Curious why I built Eurora? Read more in the <a
					href="/docs/why-eurora"
					class="text-primary underline underline-offset-4 hover:text-primary/80"
					>Why Eurora</a
				> guide.
			</p>
		</section>
	</div>
</div>
