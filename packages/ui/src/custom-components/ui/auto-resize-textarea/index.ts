import Root from './auto-resize-textarea.svelte';

type FormAutoResizeTextareaEvent<T extends Event = Event> = T & {
	currentTarget: EventTarget & HTMLTextAreaElement;
};

type AutoResizeTextareaEvents = {
	blur: FormAutoResizeTextareaEvent<FocusEvent>;
	change: FormAutoResizeTextareaEvent<Event>;
	click: FormAutoResizeTextareaEvent<MouseEvent>;
	focus: FormAutoResizeTextareaEvent<FocusEvent>;
	keydown: FormAutoResizeTextareaEvent<KeyboardEvent>;
	keypress: FormAutoResizeTextareaEvent<KeyboardEvent>;
	keyup: FormAutoResizeTextareaEvent<KeyboardEvent>;
	mouseover: FormAutoResizeTextareaEvent<MouseEvent>;
	mouseenter: FormAutoResizeTextareaEvent<MouseEvent>;
	mouseleave: FormAutoResizeTextareaEvent<MouseEvent>;
	paste: FormAutoResizeTextareaEvent<ClipboardEvent>;
	input: FormAutoResizeTextareaEvent<InputEvent>;
};

export {
	Root,
	//
	Root as AutoResizeTextarea,
	type AutoResizeTextareaEvents,
	type FormAutoResizeTextareaEvent
};
