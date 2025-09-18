import { Schema } from 'prosemirror-model';
import { default as Editor } from './Editor.svelte';
import type { SveltePMExtension, ExtensionData, Initialized } from '$lib/typings/index.js';
import { createNodeSpec } from './extensions/createNodeSpec.js';
import { keymap } from 'prosemirror-keymap';
import { schema as defaultSchema } from 'prosemirror-schema-basic';
import { addListNodes } from 'prosemirror-schema-list';
import { type Command, Plugin } from 'prosemirror-state';
import { caretAfterChip } from './plugins/caretAfterChip';

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
			// TODO: make sure the node is not duplicate
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

	// const schema = new Schema({
	// 	nodes: addListNodes(defaultSchema.spec.nodes, 'paragraph block*', 'block').append(nodes),
	// 	marks: defaultSchema.spec.marks
	// });

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
