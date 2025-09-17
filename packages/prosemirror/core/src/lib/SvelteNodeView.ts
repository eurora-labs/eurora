import { type Attrs, DOMSerializer, Node as PMNode } from 'prosemirror-model';
import { mount, type Component } from 'svelte';
import { default as Editor } from './Editor.svelte';
import type {
	Decoration,
	DecorationSource,
	EditorView,
	NodeView,
	NodeViewConstructor,
	ViewMutationRecord,
} from 'prosemirror-view';

export interface SvelteNodeViewProps<A extends Attrs> {
	node: PMNode;
	attrs: A;
	contentDOM: (node: HTMLElement) => void;
	selected: boolean | undefined;
	view: EditorView;
	getPos: () => number | undefined;
	decorations: readonly Decoration[];
	innerDecorations: DecorationSource;
	editor: Editor;
}

export interface MountedComponent extends Component {
	destroy(): void;
}

export class SvelteNodeView<A extends Attrs> implements NodeView {
	protected _dom?: HTMLElement;
	contentDOM?: HTMLElement;

	node: PMNode;
	decorations: readonly Decoration[];
	innerDecorations: DecorationSource;

	selected: boolean | undefined;
	editor: Editor;
	component?: Component<SvelteNodeViewProps<A>>;
	mounted?: MountedComponent;

	constructor(
		node: PMNode,
		readonly view: EditorView,
		readonly getPos: () => number | undefined,
		decorations: readonly Decoration[],
		innerDecorations: DecorationSource,
		editor: Editor,
		component?: Component<SvelteNodeViewProps<A>>,
	) {
		this.node = node;
		this.view = view;
		this.decorations = decorations;
		this.innerDecorations = innerDecorations;
		this.editor = editor;
		this.component = component;
	}

	get dom() {
		if (!this._dom) {
			throw Error(
				'@my-org/core: Accessing uninitialized dom inside SvelteNodeView! Check your "init" method',
			);
		}
		return this._dom;
	}

	get props(): SvelteNodeViewProps<A> {
		return {
			node: this.node,
			attrs: this.node.attrs as A,
			selected: this.selected,
			view: this.view,
			getPos: this.getPos,
			decorations: this.decorations,
			innerDecorations: this.innerDecorations,
			editor: this.editor,
			contentDOM: (node: HTMLElement) => {
				if (this.contentDOM) {
					node.appendChild(this.contentDOM);
				}
			},
		};
	}

	init = (): this => {
		const toDOM = this.node.type.spec.toDOM;
		if (!toDOM)
			throw Error(`@my-org/core: node "${this.node.type}" was not given a toDOM method!`);
		const { contentDOM } = DOMSerializer.renderSpec(document, toDOM(this.node));
		// this._dom = dom as HTMLElement
		this.contentDOM = contentDOM;
		this._dom = document.createElement(this.node.type.spec.inline ? 'span' : 'div');
		// this._dom.style.display = 'inline-flex';
		// this._dom.setAttribute('contenteditable', 'false');

		if (this.component) {
			// Mount the component synchronously but don't wait for it to complete
			// This allows init() to return immediately while the component mounts in the background
			this.mounted = mount(this.component!, {
				target: this.dom,
				props: this.props,
			}) as MountedComponent;
		} else {
			this._dom.appendChild(contentDOM!);
		}
		return this;
	};

	render() {}

	shouldUpdate = (node: PMNode) => {
		if (node.type !== this.node.type) {
			return false;
		} else if (node.sameMarkup(this.node)) {
			return false;
		}
		return true;
	};

	update = (
		node: PMNode,
		decorations: readonly Decoration[],
		innerDecorations: DecorationSource,
	): boolean => {
		// if (!newNode.sameMarkup(this.node)) return false
		if (node.type.name !== this.node.type.name) {
			return false;
		}
		this.node = node;
		this.decorations = decorations;
		this.innerDecorations = innerDecorations;
		this.render();
		return true;
	};

	selectNode = () => {
		this.selected = true;
		this.render();
	};

	// stopEvent = (e: Event) => {
	// 	// Allow keyboard events to reach the editor
	// 	if (
	// 		e.type === 'keydown' ||
	// 		e.type === 'keypress' ||
	// 		e.type === 'beforeinput' ||
	// 		e.type === 'input'
	// 	)
	// 		return false;
	// 	// Swallow pointer events inside the chip UI only
	// 	return this.dom.contains(e.target as Node);
	// };

	deselectNode = () => {
		this.selected = false;
		this.render();
	};

	destroy = () => {
		this.mounted?.destroy();
	};

	ignoreMutation = (_mutation: ViewMutationRecord) => {
		return this.dom.contains(_mutation.target);
	};

	static fromComponent<A extends Attrs>(
		editor: Editor,
		component?: Component<SvelteNodeViewProps<A>>,
	): NodeViewConstructor {
		return (
			node: PMNode,
			view: EditorView,
			getPos: () => number | undefined,
			decorations: readonly Decoration[],
			innerDecorations: DecorationSource,
		) => new this(node, view, getPos, decorations, innerDecorations, editor, component).init();
	}
}
