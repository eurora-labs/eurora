<script lang="ts">
	import { Editor as ProsemirrorEditor, type Query } from '@eurora/prosemirror-core/index.js';
	import { transcriptExtension } from '@eurora/ext-transcript/index.js';
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

	function addTranscriptNode() {
		// TODO: implement
		editorRef?.addTranscriptNode();
	}

	function addEquationNode() {
		if (!editorRef?.view) return;
		const state = editorRef.view.state;
		const tr = state.tr;
		const { schema } = state;
		const nodes = schema.nodes;
		tr.insert(
			1,
			nodes.equation.create(
				{
					latex: 'a^2 = \\sqrt{b^2 + c^2}',
				},
				schema.text('Mah equation'),
			),
		);
		// Try both approaches to ensure they both work
		// Approach 1: Using attrs

		// Approach 2: Using content
		// tr.insert(
		// 	9,
		// 	nodes.transcript.create(
		// 		{ id: 'transcript-2' },
		// 		schema.text('Youtube transcript with content')
		// 	)
		// );
		editorRef?.view.dispatch(tr);
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
