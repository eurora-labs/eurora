import { default as Editor } from '$lib/Editor.svelte';
import { type Cmd } from '$lib/typings/pm.js';
import { type NodeSpec, Node as PMNode, Schema } from 'prosemirror-model';
import { Plugin } from 'prosemirror-state';
import { type MarkViewConstructor, type NodeViewConstructor } from 'prosemirror-view';
import type { MarkSpec } from 'prosemirror-model';
import type { Component } from 'svelte';

export interface NodeProps<T> {
	node: PMNode;
	ref?: HTMLElement;
	attrs: T;
}

export interface Initialized extends ExtensionData {
	plugins: Plugin[];
	schema: Schema;
}

export interface ExtensionData {
	commands: { [name: string]: (...args: any[]) => Cmd };
	marks: { [name: string]: MarkSpec };
	markViews: { [name: string]: MarkViewConstructor };
	nodes: { [name: string]: NodeSpec };
	nodeViews: { [name: string]: NodeViewConstructor };
	sortedKeymaps: { [key: string]: { cmd: Cmd; priority: number }[] };
	svelteNodes: { [name: string]: SveltePMNode<any> };
}

export interface SveltePMMark {
	schema?: MarkSpec;
	markView?: MarkViewConstructor;
}

export interface SveltePMExtension {
	name: string;
	commands?: { [name: string]: (...args: any[]) => Cmd };
	keymaps?: { [key: string]: Cmd | { cmd: Cmd; priority: number }[] };
	store?: Record<string, any>;
	marks?: {
		[name: string]: SveltePMMark;
	};
	svelteNodes?: {
		[name: string]: SveltePMNode<any>;
	};
	position?: number;
	init?: (editor: Editor) => void;
	plugins?: (editor: Editor, schema: Schema) => Plugin[];
	destroy?: () => void;
}

export interface SveltePMNode<T> {
	attrs?: T;
	selectors?: string[];
	schema: NodeSpec;
	component?: Component<NodeProps<T>, Record<string, never>, ''>;
	nodeView?: (editor: Editor) => NodeViewConstructor;
}

export interface PMExtension {
	nodes: { [name: string]: NodeSpec };
}
