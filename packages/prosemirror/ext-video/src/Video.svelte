<script lang="ts" module>
	import type { NodeSpec } from 'prosemirror-model';
	import { Node as PMNode } from 'prosemirror-model';
	import type { NodeProps } from '@eurora/prosemirror-core';

	export interface Frame {
		id: string;
		ocrText?: string;
	}

	export interface VideoAttrs {
		id?: string;
		transcript?: string;
		currentFrame?: Frame;
	}

	export const videoAttrs: VideoAttrs = {
		id: undefined,
		transcript: undefined,
		currentFrame: undefined
	};

	export const videoSchema: NodeSpec = {
		attrs: Object.entries(videoAttrs).reduce(
			(acc, [key, value]) => ({ ...acc, [key]: { default: value } }),
			{}
		),
		content: 'inline+',
		group: 'inline',
		inline: true,
		defining: false,
		atom: true,
		selectable: false,

		parseDOM: [
			{
				tag: 'span.video', // Changed from figure
				getAttrs: (dom: HTMLElement | string) => {
					if (dom instanceof HTMLElement) {
						return {
							id: dom.getAttribute('id'),
							text: dom.getAttribute('data-text')
						};
					}
					return null;
				}
			}
		],
		toDOM(node: PMNode) {
			const { id, text } = node.attrs;
			return ['span', { id, class: 'video' }];
		}
	};
</script>

<script lang="ts">
	import { Badge, ContextChip, Dialog, Label, Input, Button, buttonVariants } from '@eurora/ui';
	import type { SvelteNodeViewProps } from '@eurora/prosemirror-core';
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

<Dialog.Root>
	<Dialog.Trigger class={buttonVariants({ variant: 'outline' })}>Edit Profile</Dialog.Trigger>
	<Dialog.Content class="sm:max-w-[425px]">
		<Dialog.Header>
			<Dialog.Title>Edit profile</Dialog.Title>
			<Dialog.Description>
				Make changes to your profile here. Click save when you're done.
			</Dialog.Description>
		</Dialog.Header>
		<div class="grid gap-4 py-4">
			<div class="grid grid-cols-4 items-center gap-4">
				<Label for="name" class="text-right">Name</Label>
				<Input id="name" value="Pedro Duarte" class="col-span-3" />
			</div>
			<div class="grid grid-cols-4 items-center gap-4">
				<Label for="username" class="text-right">Username</Label>
				<Input id="username" value="@peduarte" class="col-span-3" />
			</div>
		</div>
		<Dialog.Footer>
			<Button type="submit">Save changes</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
<ContextChip onclick={handleClick} bind:ref data-hole {...attrs} onkeydown={handleKeyDown}>
	video
</ContextChip>
