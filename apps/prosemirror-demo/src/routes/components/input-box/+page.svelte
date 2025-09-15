<script lang="ts">
	// import { LauncherNative as Launcher } from '@eurora/launcher';
	import * as Launcher from '@eurora/prosemirror-view/launcher';
	import { Editor as ProsemirrorEditor } from '@eurora/prosemirror-core/index';
	let editorRef: ProsemirrorEditor | undefined = $state(undefined);

	let exampleInput = $state({
		text: '',
		extensions: [],
	});

	function addExerciseSheet() {
		editorRef?.cmd((state, dispatch) => {
			const tr = state.tr;
			const { schema } = state;
			const nodes = schema.nodes;
			const { $from: from } = state.selection;
			tr.insert(
				from.pos,
				nodes.transcript.createChecked(
					{ id: 'transcript-1', text: 'Exercise Sheet 2' },
					schema.text(' '),
				),
			);

			dispatch?.(tr);
		});
	}
</script>

<div>
	<div class="launcher absolute top-1/4 left-1/2 w-[1100px] -translate-x-1/2">
		<Launcher.Root class="rounded-lg border shadow-md">
			<Launcher.Input placeholder="Search" bind:query={exampleInput} bind:editorRef />
		</Launcher.Root>
	</div>
	<img class=" w-full" src="/sample_background.jpg" alt="Sample Background" />
</div>

<style lang="postcss">
	/* @reference "@eurora/ui/main.css"; */
	.launcher {
		backdrop-filter: blur(36px);
		-webkit-backdrop-filter: blur(36px);
		background: rgba(255, 255, 255, 0.2);
		box-shadow: 0 4px 30px rgba(0, 0, 0, 0.1);
	}
</style>
