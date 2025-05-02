<script lang="ts">
	// import { Search } from '@lucide/svelte';
	import { Input } from '@eurora/ui';
</script>

<div>
	<div
		class="launcher absolute left-1/2 top-1/4 h-[150px] w-[1100px] -translate-x-1/2 -translate-y-1/4"
	>
		<!-- <Search /> -->
		<Input type="text" placeholder="Eurora Search" class="h-full w-full border-none text-[32px]" />
	</div>
	<img class=" w-full" src="/sample_background.jpg" alt="Sample Background" />
</div>

<style>
	.launcher {
		background: rgba(255, 255, 255, 0.2);
		box-shadow: 0 4px 30px rgba(0, 0, 0, 0.1);
		backdrop-filter: blur(18px);
		-webkit-backdrop-filter: blur(18px);
	}

	:global(input) {
		font-size: 50px !important;
		opacity: 0.5 !important;
		color: black !important;
	}
	:global(input::placeholder) {
		font-size: 50px !important;
		opacity: 0.3 !important;
		color: black !important;
	}
</style>
