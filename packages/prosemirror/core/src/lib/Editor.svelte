<script lang="ts" module>
	import type { Query } from '$lib/typings/index.js';
	import type { ClassValue } from 'svelte/elements';

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
	import { type Commands, commands as defaultCommands } from '$lib/commands.js';
	import { paragraphExtension } from '$lib/components/paragraph/extension.js';
	// eslint-disable-next-line import-x/no-cycle
	import { createExtensions } from '$lib/createExtensions.js';
	import { Node as PMNode } from 'prosemirror-model';
	import { DOMParser } from 'prosemirror-model';
	import { EditorState, Plugin } from 'prosemirror-state';
	import { EditorView } from 'prosemirror-view';
	import { onDestroy } from 'svelte';
	import type { SveltePMExtension } from '$lib/typings/extension.js';
	import type { Cmd } from '$lib/typings/index.js';

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
		if (queryParam) {
			query = queryParam;
		}

		const doc = document.createElement('p');
		doc.textContent = currentQuery?.text ?? '';

		const extensions = [...(currentQuery?.extensions ?? [])];

		if (!extensions.some((ext) => ext.name === 'paragraph')) {
			extensions.unshift(paragraphExtension());
		}

		currentExtensions = extensions;

		// @ts-expect-error - This component needs to be passed as editor context
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
		const newExtensions = [...(newQuery.extensions ?? [])];

		if (!newExtensions.some((ext) => ext.name === 'paragraph')) {
			newExtensions.unshift(paragraphExtension());
		}

		const extensionsChanged = hasExtensionsChanged(currentExtensions, newExtensions);

		if (!extensionsChanged) {
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

				query = newQuery;
			}
			return;
		}

		currentExtensions = newExtensions;
		query = newQuery;

		// @ts-expect-error - This component needs to be passed as editor context
		const created = await createExtensions(this as any, newExtensions);

		const currentState = view.state;

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

		view.updateState(newState);

		view.setProps({
			nodeViews: created.nodeViews,
			markViews: created.markViews,
		});
	}

	function hasExtensionsChanged(
		oldExts: SveltePMExtension[],
		newExts: SveltePMExtension[],
	): boolean {
		if (oldExts.length !== newExts.length) return true;

		const oldExtMap = new Map(oldExts.map((ext) => [ext.name, ext]));
		const newExtMap = new Map(newExts.map((ext) => [ext.name, ext]));

		for (const [name] of oldExtMap) {
			if (!newExtMap.has(name)) return true;
		}

		for (const [name] of newExtMap) {
			if (!oldExtMap.has(name)) return true;
		}

		return false;
	}

	export async function sendQuery(newQuery?: Query) {
		if (!view) {
			await init(newQuery);
		}
	}

	export function cmd(command: Cmd) {
		if (!view) return;
		command(view.state, view.dispatch, view);
		commands.focus()(view.state, view.dispatch, view);
	}

	$effect(() => {
		if (editorRef && onheightchange) {
			resizeObserver?.disconnect();

			resizeObserver = new ResizeObserver((entries) => {
				for (const entry of entries) {
					const height = entry.contentRect.height;
					onheightchange(height);
				}
			});
			resizeObserver.observe(editorRef);
		}

		return () => {
			resizeObserver?.disconnect();
		};
	});

	onDestroy(() => {
		view?.destroy();
		resizeObserver?.disconnect();
	});

	function placeholderPlugin(text: string) {
		function update(view: EditorView) {
			if (view.state.doc.content.size > 2) {
				editorRef?.removeAttribute('data-placeholder');
			} else {
				editorRef?.setAttribute('data-placeholder', text);
			}
		}

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
		white-space: pre-wrap;
		overflow-wrap: break-word;
	}

	:global(.ProseMirror[data-placeholder])::before {
		position: absolute;
		content: attr(data-placeholder);
		color: rgba(255, 255, 255, 0.2);
		pointer-events: none;
	}
</style>
