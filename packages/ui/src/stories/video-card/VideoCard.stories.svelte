<script module lang="ts">
	import {
		Card,
		CardContent,
		CardHeader,
		CardTitle,
		CardDescription
	} from '$lib/custom-components/video-card/index.js';
	import { defineMeta } from '@storybook/addon-svelte-csf';

	const { Story } = defineMeta({
		title: 'Components / VideoCard',
		component: Card,
		parameters: {
			docs: {
				description: {
					component:
						'A responsive video card component with flexible layout. Features video content with text overlay, supporting both left and right alignment. Automatically adapts from column layout on mobile to row layout on desktop.'
				}
			},
			layout: 'padded'
		}
	});
</script>

<!-- Default Video Card -->
<Story name="Default">
	<Card>
		<CardContent
			mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
		>
			<CardHeader>
				<CardTitle>Amazing Video Content</CardTitle>
				<CardDescription>
					Discover incredible stories and experiences through our curated video content.
					This responsive card adapts beautifully to any screen size.
				</CardDescription>
			</CardHeader>
		</CardContent>
	</Card>
</Story>

<!-- Left Alignment -->
<Story name="Left Alignment">
	<Card>
		<CardContent
			alignment="left"
			mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4"
		>
			<CardHeader>
				<CardTitle>Left Aligned Video</CardTitle>
				<CardDescription>
					Video content positioned on the left side with text content on the right.
					Perfect for showcasing product demos or feature highlights.
				</CardDescription>
			</CardHeader>
		</CardContent>
	</Card>
</Story>

<!-- Right Alignment -->
<Story name="Right Alignment">
	<Card>
		<CardContent
			alignment="right"
			mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerBlazes.mp4"
		>
			<CardHeader>
				<CardTitle>Right Aligned Video</CardTitle>
				<CardDescription>
					Video content positioned on the right side with text content on the left. Great
					for alternating layouts in content sections.
				</CardDescription>
			</CardHeader>
		</CardContent>
	</Card>
</Story>

<!-- Multiple Video Formats -->
<Story name="Multiple Formats">
	<Card>
		<CardContent
			mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
			webmSrc="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.webm"
		>
			<CardHeader>
				<CardTitle>Multi-Format Support</CardTitle>
				<CardDescription>
					This video card supports multiple video formats for optimal browser
					compatibility. WebM format provides better compression while MP4 ensures broad
					support.
				</CardDescription>
			</CardHeader>
		</CardContent>
	</Card>
</Story>

<!-- Long Content -->
<Story name="Long Content">
	<Card>
		<CardContent
			mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/SubaruOutbackOnStreetAndDirt.mp4"
		>
			<CardHeader>
				<CardTitle>Extended Content Example</CardTitle>
				<CardDescription>
					This example demonstrates how the video card handles longer text content. The
					flexible layout ensures that both video and text content remain readable and
					well-proportioned regardless of content length. The responsive design
					automatically adjusts spacing and alignment to maintain visual hierarchy and
					optimal user experience across all device sizes.
				</CardDescription>
			</CardHeader>
		</CardContent>
	</Card>
</Story>

<!-- Custom Styling -->
<Story name="Custom Styling">
	<Card
		class="bg-gradient-to-br from-purple-50 to-blue-50 dark:from-purple-900/20 dark:to-blue-900/20"
	>
		<CardContent
			mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/TearsOfSteel.mp4"
			class="gap-8"
		>
			<CardHeader>
				<CardTitle class="text-purple-600 dark:text-purple-400"
					>Custom Styled Card</CardTitle
				>
				<CardDescription class="text-purple-700/80 dark:text-purple-300/80">
					Video cards can be customized with different backgrounds, colors, and spacing.
					This example shows a gradient background with custom text colors.
				</CardDescription>
			</CardHeader>
		</CardContent>
	</Card>
</Story>

<!-- Minimal Content -->
<Story name="Minimal">
	<Card>
		<CardContent
			mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/VolkswagenGTIReview.mp4"
		>
			<CardHeader>
				<CardTitle>Simple Title</CardTitle>
			</CardHeader>
		</CardContent>
	</Card>
</Story>

<!-- No Video Source (Fallback) -->
<Story name="No Video Source">
	<Card>
		<CardContent>
			<CardHeader>
				<CardTitle>No Video Available</CardTitle>
				<CardDescription>
					This demonstrates the fallback behavior when no video source is provided. The
					layout still works perfectly for text-only content.
				</CardDescription>
			</CardHeader>
		</CardContent>
	</Card>
</Story>

<!-- Component Composition -->
<Story name="Component Composition">
	<div class="space-y-8">
		<h2 class="text-2xl font-bold">Individual Components</h2>

		<div class="space-y-4">
			<h3 class="text-lg font-semibold">Card Container</h3>
			<Card class="p-8">
				<p>This is just the card container without any content.</p>
			</Card>
		</div>

		<div class="space-y-4">
			<h3 class="text-lg font-semibold">Card Header</h3>
			<CardHeader>
				<CardTitle>Standalone Title</CardTitle>
				<CardDescription>This header can be used independently.</CardDescription>
			</CardHeader>
		</div>

		<div class="space-y-4">
			<h3 class="text-lg font-semibold">Card Title Only</h3>
			<CardTitle>Large Standalone Title</CardTitle>
		</div>

		<div class="space-y-4">
			<h3 class="text-lg font-semibold">Card Description Only</h3>
			<CardDescription>
				This is a standalone description component that can be used for subtitles or
				explanatory text.
			</CardDescription>
		</div>
	</div>
</Story>

<!-- Responsive Layout Demo -->
<Story name="Responsive Layout">
	<div class="space-y-8">
		<div class="text-center">
			<h2 class="mb-4 text-2xl font-bold">Responsive Behavior</h2>
			<p class="text-muted-foreground">
				Resize your browser window to see how the layout adapts from desktop to mobile
			</p>
		</div>

		<Card>
			<CardContent
				alignment="left"
				mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4"
			>
				<CardHeader>
					<CardTitle>Desktop: Side by Side</CardTitle>
					<CardDescription>
						On desktop screens, the video and text appear side by side. The video takes
						up 3/5 of the width while text takes 2/5.
					</CardDescription>
				</CardHeader>
			</CardContent>
		</Card>

		<Card>
			<CardContent
				alignment="right"
				mp4Src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ElephantsDream.mp4"
			>
				<CardHeader>
					<CardTitle>Mobile: Stacked Layout</CardTitle>
					<CardDescription>
						On mobile devices, the content stacks vertically with text on top and video
						below for optimal readability and user experience.
					</CardDescription>
				</CardHeader>
			</CardContent>
		</Card>
	</div>
</Story>
