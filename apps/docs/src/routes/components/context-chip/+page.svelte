<script lang="ts">
	import { ContextChip } from '@eurora/ui';
</script>

<div class="flex flex-col gap-4 p-4">
	<h1 class="text-2xl font-bold">Context Chip Component</h1>

	<div class="flex flex-wrap gap-4">
		<div>
			<h2 class="mb-2 text-lg font-semibold">Default</h2>
			<ContextChip>Default Context</ContextChip>
		</div>

		<div>
			<h2 class="mb-2 text-lg font-semibold">Primary</h2>
			<ContextChip variant="primary">Primary Context</ContextChip>
		</div>

		<div>
			<h2 class="mb-2 text-lg font-semibold">Secondary</h2>
			<ContextChip variant="secondary">Secondary Context</ContextChip>
		</div>

		<div>
			<h2 class="mb-2 text-lg font-semibold">Destructive</h2>
			<ContextChip variant="destructive">Destructive Context</ContextChip>
		</div>

		<div>
			<h2 class="mb-2 text-lg font-semibold">Outline</h2>
			<ContextChip variant="outline">Outline Context</ContextChip>
		</div>
	</div>

	<div class="mt-4">
		<h2 class="mb-2 text-lg font-semibold">With Click Handler</h2>
		<ContextChip onClick={(e) => alert('Context chip clicked!')}>Clickable Context</ContextChip>
	</div>

	<div class="mt-4">
		<h2 class="mb-2 text-lg font-semibold">With Link</h2>
		<ContextChip href="#" onClick={(e) => e.preventDefault()}>Link Context</ContextChip>
	</div>
</div>
