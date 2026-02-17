import { default as Editor } from '$lib/Editor.svelte';
import { createNodeSpec } from '$lib/extensions/createNodeSpec.js';
import { caretAfterChip } from '$lib/plugins/caretAfterChip';
import { keymap } from 'prosemirror-keymap';
import { Schema } from 'prosemirror-model';
import { type Command, Plugin } from 'prosemirror-state';
import type { SveltePMExtension, ExtensionData, Initialized } from '$lib/typings/index.js';

export async function createExtensions(
	editor: Editor,
	extensions: SveltePMExtension[] = [],
): Promise<Initialized> {
	const extData: ExtensionData = {
		commands: {},
		marks: {},
		markViews: {},
		nodes: {},
		nodeViews: {},
		sortedKeymaps: {},
		svelteNodes: {},
	};
	for (const ext of extensions) {
		for (const name in ext.keymaps) {
			const val = ext.keymaps[name];
			const cmd = Array.isArray(val) ? val : [{ cmd: val, priority: 0 }];
			cmd.sort((a, b) => b.priority - a.priority);
			if (name in extData.sortedKeymaps) {
				extData.sortedKeymaps[name] = [...extData.sortedKeymaps[ext.name], ...cmd].sort(
					(a, b) => b.priority - a.priority,
				);
			} else {
				extData.sortedKeymaps[name] = cmd;
			}
		}

		for (const nodeKey in ext.svelteNodes) {
			const node = ext.svelteNodes[nodeKey];
			extData.nodes[nodeKey] = await createNodeSpec(node);
			if (node.nodeView) {
				extData.nodeViews[nodeKey] = node.nodeView(editor);
			}
		}

		for (const name in ext.marks) {
			if (name in extData.marks) {
				throw Error(
					`@my-org/core: duplicate mark "${name}" provided from extension ${ext.name}`,
				);
			}
			const { schema, markView } = ext.marks[name];
			if (schema) {
				extData.marks[name] = schema;
			}
			if (markView) {
				extData.markViews[name] = markView;
			}
		}

		if (ext.commands) {
			extData.commands = { ...extData.commands, ...ext.commands };
		}
	}

	const schema = new Schema({
		nodes: {
			doc: {
				content: 'block+',
			},
			text: {
				group: 'inline',
			},
			...extData.nodes,
		},
		marks: extData.marks,
	});

	const keymaps = Object.keys(extData.sortedKeymaps).reduce(
		(acc, key) => {
			// @ts-expect-error extData.sortedKeymaps[key][0].cmd is not a Command
			acc[key] = extData.sortedKeymaps[key][0].cmd;
			return acc;
		},
		{} as { [key: string]: Command },
	);

	const plugins = [
		keymap(keymaps),
		caretAfterChip,
		...extensions.reduce(
			(acc, ext) => [...acc, ...((ext.plugins && ext.plugins(editor, schema)) || [])],
			[] as Plugin[],
		),
	];

	return {
		...extData,
		schema,
		plugins,
	};
}
