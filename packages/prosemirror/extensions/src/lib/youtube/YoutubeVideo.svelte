<script lang="ts" module>
	import type { NodeSpec } from 'prosemirror-model';
	import { Node as PMNode } from 'prosemirror-model';

	export interface Frame {
		id: string;
		ocrText?: string;
	}

	export interface VideoAttrs {
		id?: string;
		transcript?: string;
		text?: string;
		name?: string;
		currentFrame?: Frame;
	}

	export const videoAttrs: VideoAttrs = {
		id: undefined,
		transcript: undefined,
		text: undefined,
		name: undefined,
		currentFrame: undefined,
	};

	export const videoSchema: NodeSpec = {
		attrs: Object.entries(videoAttrs).reduce(
			(acc, [key, value]) => ({ ...acc, [key]: { default: value } }),
			{},
		),
		group: 'inline',
		inline: true,
		atom: true,
		selectable: true,

		parseDOM: [
			{
				tag: 'span.video', // Changed from figure
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return {
							id: dom.getAttribute('id'),
							text: dom.getAttribute('data-text'),
							name: dom.getAttribute('data-name'),
						};
					}
					return null;
				},
			},
		],
		toDOM(node: PMNode) {
			const { id, text, name } = node.attrs;
			return ['span', { id, 'data-text': text, 'data-name': name, class: 'video' }];
		},
	};
</script>

<script lang="ts">
	import { ContextChip } from '@eurora/ui/custom-components/context-chip/index';
	import { Label } from '@eurora/ui/components/label/index';
	import { Input } from '@eurora/ui/components/input/index';
	import * as Popover from '@eurora/ui/components/popover/index';
	import type { SvelteNodeViewProps } from '@eurora/prosemirror-core/index';
	import { SiYoutube } from '@icons-pack/svelte-simple-icons';
	export interface Props extends SvelteNodeViewProps<VideoAttrs> {
		ref: HTMLElement;
		attrs: VideoAttrs;
	}

	let { ref, attrs }: Props = $props();

	export { ref, attrs };

	function handleClick(event: MouseEvent) {
		alert('some longer script');
		event.preventDefault();
	}

	function handleKeyDown(event: KeyboardEvent) {
		event.preventDefault();
	}

	export function destroy() {
		ref?.remove();
	}
</script>

<Popover.Root>
	<Popover.Trigger>
		<ContextChip bind:ref data-hole {...attrs} onkeydown={handleKeyDown}>
			<SiYoutube size={24} />
			{attrs.name}
		</ContextChip>
	</Popover.Trigger>
	<Popover.Content class="w-80">
		<div class="grid gap-4">
			<div class="space-y-2">
				<h4 class="font-medium leading-none">Dimensions</h4>
				<p class="text-muted-foreground text-sm">Set the dimensions for the layer.</p>
			</div>
			<div class="grid gap-2">
				<div class="grid grid-cols-3 items-center gap-4">
					<Label for="width">Width</Label>
					<Input id="width" value="100%" class="col-span-2 h-8" />
				</div>
				<div class="grid grid-cols-3 items-center gap-4">
					<Label for="maxWidth">Max. width</Label>
					<Input id="maxWidth" value="300px" class="col-span-2 h-8" />
				</div>
				<div class="grid grid-cols-3 items-center gap-4">
					<Label for="height">Height</Label>
					<Input id="height" value="25px" class="col-span-2 h-8" />
				</div>
				<div class="grid grid-cols-3 items-center gap-4">
					<Label for="maxHeight">Max. height</Label>
					<Input id="maxHeight" value="none" class="col-span-2 h-8" />
				</div>
			</div>
		</div>
	</Popover.Content>
</Popover.Root>
