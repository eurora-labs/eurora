<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import { processQuery, clearQuery, type QueryAssets } from '@eurora/prosemirror-core/util';
	import * as Card from '@eurora/ui/components/card/index';
	import * as Launcher from '@eurora/prosemirror-view/launcher';
	import {
		Editor as ProsemirrorEditor,
		type SveltePMExtension,
	} from '@eurora/prosemirror-core/index';
	import { extensionFactory, registerCoreExtensions } from '@eurora/prosemirror-factory/index';

	let editorRef: ProsemirrorEditor | undefined = $state(undefined);

	registerCoreExtensions();
	let searchQuery = $state({
		text: '',
		extensions: [
			// extensionFactory.getExtension('9370B14D-B61C-4CE2-BDE7-B18684E8731A'),
			extensionFactory.getExtension('7c7b59bb-d44d-431a-9f4d-64240172e092'),
			extensionFactory.getExtension('309f0906-d48c-4439-9751-7bcf915cdfc5'),
			extensionFactory.getExtension('2c434895-d32c-485f-8525-c4394863b83a'),
		] as SveltePMExtension[],
	});

	function addArticleChip() {
		editorRef?.cmd((state, dispatch) => {
			const tr = state.tr;
			const { schema } = state;
			const nodes = schema.nodes;
			const { $from: from } = state.selection;
			console.log(nodes);
			tr.insert(
				from.pos,
				nodes['309f0906-d48c-4439-9751-7bcf915cdfc5'].createChecked({
					id: 'article-1',
					name: 'article',
					text: 'Article 1',
				}),
			);
			dispatch?.(tr);
		});
	}

	function printQueryToConsole() {
		if (!editorRef) return;

		console.log(processQuery(editorRef));
	}

	// function addExerciseSheet() {
	// 	editorRef?.cmd((state, dispatch) => {
	// 		const tr = state.tr;
	// 		const { schema } = state;
	// 		const nodes = schema.nodes;
	// 		const { $from: from } = state.selection;
	// 		tr.insert(
	// 			from.pos,
	// 			nodes.transcript.createChecked(
	// 				{ id: 'transcript-1', text: 'Exercise Sheet 2' },
	// 				schema.text(' '),
	// 			),
	// 		);

	// 		dispatch?.(tr);
	// 	});
	// }
</script>

<div class="flex flex-col">
	<div class="w-full h-1/2 justify-center items-center">
		<div class="launcher absolute top-1/4 left-1/2 w-[1100px] -translate-x-1/2">
			<Launcher.Root class="rounded-lg border shadow-md min-h-[100px]">
				<Launcher.Input placeholder="Search" bind:query={searchQuery} bind:editorRef />
			</Launcher.Root>
		</div>
		<img class=" w-full" src="/sample_background.jpg" alt="Sample Background" />
	</div>

	<div class="w-full h-1/2 bg-white flex flex-row gap-4">
		<div class="flex flex-col gap-4 w-fit">
			<h2 class="text-2xl font-bold">Context Chips Controls</h2>
			<Button onclick={addArticleChip}>Add article</Button>
			<Button>Add youtube</Button>
		</div>
		<div class="flex flex-col gap-4 w-fit">
			<h2 class="text-2xl font-bold">Debugging Controls</h2>

			<Button onclick={printQueryToConsole}>Print query to console</Button>
		</div>
	</div>
</div>

<style lang="postcss">
	.launcher {
		backdrop-filter: blur(36px);
		-webkit-backdrop-filter: blur(36px);
		background: rgba(255, 255, 255, 0.2);
		box-shadow: 0 4px 30px rgba(0, 0, 0, 0.1);
	}
</style>
