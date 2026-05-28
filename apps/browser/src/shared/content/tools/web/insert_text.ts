import { writableFieldKind } from '../../extensions/web/element-filter';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z
	.object({
		field_id: z.string().min(1),
		text: z.string(),
		replace: z.boolean().optional(),
	})
	.strict();

const Out = z.object({
	field_id: z.string(),
	previous_value: z.string(),
	new_value: z.string(),
});

type Result = z.infer<typeof Out>;

const CONTENT_EDITABLE_VALUES = new Set(['', 'true', 'plaintext-only']);

function isLikelyContentEditable(el: Element): el is HTMLElement {
	if (!(el instanceof HTMLElement)) return false;
	const attr = el.getAttribute('contenteditable');
	return attr !== null && CONTENT_EDITABLE_VALUES.has(attr.toLowerCase());
}

function readCurrentValue(el: Element): string {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) return el.value;
	return el.textContent ?? '';
}

function setNativeValue(el: Element, prototype: object, value: string): void {
	const descriptor = Object.getOwnPropertyDescriptor(prototype, 'value');
	const setter = descriptor?.set;
	if (setter) {
		setter.call(el, value);
		return;
	}
	(el as unknown as { value: string }).value = value;
}

function writeValue(el: Element, value: string): void {
	if (el instanceof HTMLInputElement) {
		setNativeValue(el, HTMLInputElement.prototype, value);
		return;
	}
	if (el instanceof HTMLTextAreaElement) {
		setNativeValue(el, HTMLTextAreaElement.prototype, value);
		return;
	}
	if (isLikelyContentEditable(el)) {
		el.textContent = value;
		return;
	}
	throw new Error(`insert_text refused to write into <${el.localName}>`);
}

function dispatchInputEvent(el: Element): void {
	const target = el as HTMLElement;
	const event = new (target.ownerDocument?.defaultView?.InputEvent ?? InputEvent)('input', {
		bubbles: true,
		cancelable: true,
	});
	target.dispatchEvent(event);
}

/// Safety-contract violation. Surfaced as a thrown `Error` so the
/// framework's `invokeFrom` maps it onto `ToolErrorWire::Adapter`. The
/// model sees a human-readable explanation rather than a transport
/// failure.
class SafetyViolation extends Error {
	constructor(message: string) {
		super(`safety violation: ${message}`);
		this.name = 'SafetyViolation';
	}
}

export async function executeInsertText(args: z.infer<typeof Args>): Promise<Result> {
	let targets: NodeListOf<Element>;
	try {
		targets = document.querySelectorAll(args.field_id);
	} catch (err) {
		const detail = err instanceof Error ? err.message : String(err);
		throw new SafetyViolation(
			`field_id "${args.field_id}" is not a valid CSS selector: ${detail}`,
		);
	}

	if (targets.length === 0) {
		throw new SafetyViolation(`field_id "${args.field_id}" matched zero elements`);
	}
	if (targets.length > 1) {
		throw new SafetyViolation(
			`field_id "${args.field_id}" matched ${targets.length} elements; must be unique`,
		);
	}

	const target = targets[0];
	if (!writableFieldKind(target)) {
		throw new SafetyViolation(
			`field_id "${args.field_id}" is not a writable text field (must be a text-typed <input>, <textarea>, or [contenteditable])`,
		);
	}

	const previous = readCurrentValue(target);
	const next = args.replace === true ? args.text : `${previous}${args.text}`;
	writeValue(target, next);
	dispatchInputEvent(target);

	return {
		field_id: args.field_id,
		previous_value: previous,
		new_value: next,
	};
}

export const insertText: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'web_insert_text',
		description:
			'The only mutating web tool: insert text into a uniquely-identified writable text field (<input>, <textarea>, or [contenteditable]). `replace=true` overwrites the previous value; otherwise the text is appended. Password, file, submit, and other non-text inputs are rejected; multiple-match selectors are rejected. Never dispatches `change`, `submit`, `keydown`, or focus/blur events.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 2_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return await executeInsertText(args);
	},
};
