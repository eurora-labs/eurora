<script lang="ts">
	import { Editor as ProsemirrorEditor, type Query } from '@eurora/prosemirror-core/index';
	import { transcriptExtension } from '@eurora/ext-transcript/index';
	import { onMount } from 'svelte';

	let editorRef: ProsemirrorEditor | null = $state(null);

	const exampleInput: Query = {
		text: 'Where in ',
		extensions: [],
	};

	onMount(() => {
		editorRef?.sendQuery(exampleInput);
	});

	// const exampleInput =
	// 	'Where in <transcript text="Some Transcript"/> is the following topic discussed <google-drive title="Some file"/>';

	function clear() {
		console.log(editorRef);
		// editorRef?.clear();
	}

	function focus() {
		// editorRef?.focus();
	}

	async function addTranscriptExtension() {
		const position = 9;
		exampleInput.extensions.push({ ...transcriptExtension(), position });
		await editorRef?.sendQuery(exampleInput);
		editorRef?.cmd((state, dispatch) => {
			const tr = state.tr;
			const { schema } = state;
			const nodes = schema.nodes;
			tr.insert(
				position,
				nodes.transcript.createChecked(
					{ id: 'transcript-1', text: 'Some transcript with attrs' },
					schema.text('transcript'),
				),
			);
			dispatch?.(tr);
		});
	}
</script>

<ProsemirrorEditor bind:this={editorRef} />

<div class="controls">
	<button onclick={clear}>Clear</button>
	<button>Reset</button>
	<button>Select all</button>
	<button onclick={focus}>Focus</button>
	<button onclick={addTranscriptExtension}>Add transcript</button>
</div>

<!-- <div class="mirror">Current plain text content of the editor: "{textContent}"</div> -->
