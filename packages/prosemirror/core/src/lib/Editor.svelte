<script lang="ts" module>
	import type { ClassValue } from 'svelte/elements';
	import type { Query } from '$lib/typings/index.js';

	export interface EditorProps {
		value?: string;
		query?: Query;
		placeholder?: string;
		class?: ClassValue;
		onkeydown?: (event: KeyboardEvent) => void;
		onheightchange?: (height: number) => void;
	}
</script>

<script lang="ts">
	import { EditorState, Plugin } from 'prosemirror-state';
	import { Node as PMNode } from 'prosemirror-model';
	import { DOMParser } from 'prosemirror-model';
	import { EditorView } from 'prosemirror-view';
	import type { Cmd } from '$lib/typings/index.js';
	import { onDestroy } from 'svelte';
	import type { SveltePMExtension } from '$lib/typings/extension.js';
	import { createExtensions } from '$lib/createExtensions.js';
	import { paragraphExtension } from '$lib/components/paragraph/extension.js';
	import { type Commands, commands as defaultCommands } from '$lib/commands.js';
	// import '$lib/Editor.css';

	let editorRef: HTMLDivElement | null = $state(null);
	let view: EditorView | null = null;
	let currentExtensions: SveltePMExtension[] = [];
	let commands: Commands = $state(defaultCommands);
	let mainNode: PMNode | null = null;
	let resizeObserver: ResizeObserver | null = null;

	let {
		value = $bindable(''),
		query,
		placeholder = 'Type something',
		onkeydown,
		onheightchange,
		class: className,
	}: EditorProps = $props();

	export async function init(queryParam?: Query) {
		if (!editorRef) return;
		const currentQuery = queryParam || query;
		// Update the query reference if a new one is provided
		if (queryParam) {
			query = queryParam;
		}

		const doc = document.createElement('p');
		doc.textContent = currentQuery?.text ?? '';

		// Make a copy of the extensions to avoid modifying the original
		const extensions = [...(currentQuery?.extensions ?? [])];

		// Add paragraph extension if not already present
		if (!extensions.some((ext) => ext.name === 'paragraph')) {
			extensions.unshift(paragraphExtension());
		}

		// Store the current extensions
		currentExtensions = extensions;

		// @ts-ignore
		const created = await createExtensions(this as any, extensions);
		mainNode = DOMParser.fromSchema(created.schema).parse(doc);

		view = new EditorView(
			{
				mount: editorRef,
			},
			{
				state: EditorState.create({
					schema: created.schema,
					plugins: [...created.plugins, placeholderPlugin(placeholder)],
					doc: mainNode,
				}),
				nodeViews: created.nodeViews,
				markViews: created.markViews,
			},
		);
		view.focus();
		editorRef.focus();
	}

	export async function updateExtensions(newQuery: Query) {
		if (!view || !editorRef) return;
		// Make a copy of the extensions to avoid modifying the original
		const newExtensions = [...(newQuery.extensions ?? [])];

		// Add paragraph extension if not already present
		if (!newExtensions.some((ext) => ext.name === 'paragraph')) {
			newExtensions.unshift(paragraphExtension());
		}

		// Check if extensions have changed
		const extensionsChanged = hasExtensionsChanged(currentExtensions, newExtensions);

		if (!extensionsChanged) {
			// If extensions haven't changed, just update the content if needed
			if (newQuery.text !== query?.text) {
				const doc = document.createElement('p');
				doc.textContent = newQuery.text;

				const state = view.state;
				const tr = state.tr;
				tr.replaceWith(
					0,
					state.doc.content.size,
					DOMParser.fromSchema(state.schema).parse(doc),
				);
				view.dispatch(tr);

				// Update the query reference
				query = newQuery;
			}
			return;
		}

		// Store the new extensions
		currentExtensions = newExtensions;

		// Update the query reference
		query = newQuery;

		// @ts-ignore
		const created = await createExtensions(this as any, newExtensions);

		// Get the current selection and doc content if we want to preserve it
		const currentState = view.state;

		// Create a new state with the updated schema and plugins
		const newState = EditorState.create({
			schema: created.schema,
			plugins: [...created.plugins, placeholderPlugin(placeholder)],
			doc: newQuery.text
				? (() => {
						const p = document.createElement('p');
						p.textContent = newQuery.text;
						return DOMParser.fromSchema(created.schema).parse(p);
					})()
				: currentState.doc,
		});

		// Update the view with the new state
		view.updateState(newState);

		// Update nodeViews and markViews
		view.setProps({
			nodeViews: created.nodeViews,
			markViews: created.markViews,
		});
	}

	// Helper function to check if extensions have changed
	function hasExtensionsChanged(
		oldExts: SveltePMExtension[],
		newExts: SveltePMExtension[],
	): boolean {
		if (oldExts.length !== newExts.length) return true;

		// Create maps of extension names for faster lookup
		const oldExtMap = new Map(oldExts.map((ext) => [ext.name, ext]));
		const newExtMap = new Map(newExts.map((ext) => [ext.name, ext]));

		// Check if any extension names differ
		for (const [name] of oldExtMap) {
			if (!newExtMap.has(name)) return true;
		}

		for (const [name] of newExtMap) {
			if (!oldExtMap.has(name)) return true;
		}

		// For extensions with the same name, we could do a deeper comparison
		// but for simplicity, we'll assume they're different if they have the same name
		// A more thorough check would compare the actual extension properties

		return false;
	}

	export async function sendQuery(newQuery?: Query) {
		if (!view) {
			await init(newQuery);
		} else {
			// await updateExtensions(newQuery);
		}
	}

	export function cmd(command: Cmd) {
		if (!view) return;
		command(view.state, view.dispatch, view);
		commands.focus()(view.state, view.dispatch, view);
	}

	// Set up resize observer when editorRef becomes available
	$effect(() => {
		if (editorRef && onheightchange) {
			// Clean up existing observer if any
			resizeObserver?.disconnect();

			resizeObserver = new ResizeObserver((entries) => {
				for (const entry of entries) {
					const height = entry.contentRect.height;
					onheightchange(height);
				}
			});
			resizeObserver.observe(editorRef);
		}

		// Cleanup function
		return () => {
			resizeObserver?.disconnect();
		};
	});

	onDestroy(() => {
		view?.destroy();
		resizeObserver?.disconnect();
	});

	function placeholderPlugin(text: string) {
		const update = (view: EditorView) => {
			if (view.state.doc.content.size > 2) {
				editorRef?.removeAttribute('data-placeholder');
			} else {
				editorRef?.setAttribute('data-placeholder', text);
			}
		};

		return new Plugin({
			view(view) {
				update(view);

				return { update };
			},
		});
	}

	export { view };
</script>

<div
	bind:textContent={value}
	spellcheck={false}
	class:ProseMirror={true}
	contenteditable
	{onkeydown}
	bind:this={editorRef}
	class={className}
	role="textbox"
	tabindex={0}
></div>

<style lang="postcss">
	:global(.context-chip) {
		@apply cursor-pointer;
	}
	:global(.ProseMirror-separator) {
		display: none;
	}
	:global(.ProseMirror-trailingBreak) {
		display: none;
	}
	:global(.ProseMirror) {
		align-items: anchor-center;
		width: 100%;
		border-top: 0;
		outline: none;
		/*line-height: 100px;*/
		white-space: pre-wrap;
		overflow-wrap: break-word;
	}

	:global(.ProseMirror[data-placeholder])::before {
		position: absolute;
		content: attr(data-placeholder);
		color: rgba(0, 0, 0, 0.2);
		/*line-height: 100px;*/
		pointer-events: none;
	}
</style>
