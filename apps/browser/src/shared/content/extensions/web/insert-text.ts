import { writableFieldKind } from './element-filter';
import type { InsertTextArgs, InsertTextResult } from '../../bindings';
import type { NativeResponse } from '../../models';
import type { BrowserObj } from '../watchers/watcher';

const CONTENT_EDITABLE_VALUES = new Set(['', 'true', 'plaintext-only']);

function isLikelyContentEditable(el: Element): el is HTMLElement {
	if (!(el instanceof HTMLElement)) {
		return false;
	}
	const attr = el.getAttribute('contenteditable');
	return attr !== null && CONTENT_EDITABLE_VALUES.has(attr.toLowerCase());
}

/**
 * Sentinel `code` value matched by `tab-rpc.ts::isSafetyViolation`.
 * Mapped by the background script to `ErrorFrame { code: 400 }` so the
 * desktop bridge surfaces it as `ToolError::Remote { code: 400, … }`.
 * Keep this string in sync with the constant in
 * `apps/browser/src/shared/background/tab-rpc.ts`.
 */
export const SAFETY_VIOLATION = 'SAFETY_VIOLATION';

/**
 * The only mutating web tool. The safety contract — enforced regardless
 * of what the model sends — is:
 *
 *   1. `field_id` resolves to exactly one element via
 *      `document.querySelectorAll`. Zero / multiple matches → reject.
 *   2. Target must be a writable text field: `<input>` with type in
 *      `{text,search,email,url,tel,number}`, `<textarea>`, or
 *      `[contenteditable]`. Password / file / submit-style inputs are
 *      hard-rejected here, not just hidden.
 *   3. `disabled` and `readonly` → reject.
 *   4. The write uses React's native-setter idiom (property descriptor
 *      on `HTMLInputElement.prototype.value`) so controlled components
 *      don't drop the value on the next render. A single bubbling
 *      `InputEvent('input')` follows so frameworks see the change.
 *   5. **Never** dispatch `change`, `keydown`, `keyup`, `keypress`,
 *      `submit`, `focus`, or `blur` — many sites submit forms on those
 *      paths, and the v1 contract bars us from triggering submission.
 *
 * Safety-contract violations are returned as a structured error
 * envelope (`{kind: 'Error', code: 'SAFETY_VIOLATION', …}`) so the
 * background script can distinguish them from internal handler bugs.
 */
export async function handleInsertText(
	obj: BrowserObj,
): Promise<InsertTextResult | NativeResponse> {
	const args = parseArgs(obj);
	if (!args) {
		return violation('insert_text requires { field_id: string, text: string }');
	}

	let targets: NodeListOf<Element>;
	try {
		targets = document.querySelectorAll(args.field_id);
	} catch (err) {
		return violation(
			`field_id "${args.field_id}" is not a valid CSS selector: ${describe(err)}`,
		);
	}

	if (targets.length === 0) {
		return violation(`field_id "${args.field_id}" matched zero elements`);
	}
	if (targets.length > 1) {
		return violation(
			`field_id "${args.field_id}" matched ${targets.length} elements; must be unique`,
		);
	}

	const target = targets[0];
	const kind = writableFieldKind(target);
	if (!kind) {
		return violation(
			`field_id "${args.field_id}" is not a writable text field (must be a text-typed <input>, <textarea>, or [contenteditable])`,
		);
	}

	const previous = readCurrentValue(target);
	const next = args.replace ? args.text : `${previous}${args.text}`;
	writeValue(target, next);
	dispatchInputEvent(target);

	return {
		field_id: args.field_id,
		previous_value: previous,
		new_value: next,
	};
}

interface ParsedArgs {
	field_id: string;
	text: string;
	replace: boolean;
}

function parseArgs(obj: BrowserObj): ParsedArgs | null {
	const args = obj as Partial<InsertTextArgs>;
	if (typeof args.field_id !== 'string' || args.field_id.length === 0) {
		return null;
	}
	if (typeof args.text !== 'string') {
		return null;
	}
	return {
		field_id: args.field_id,
		text: args.text,
		replace: args.replace === true,
	};
}

function readCurrentValue(el: Element): string {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		return el.value;
	}
	return el.textContent ?? '';
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
	// `writableFieldKind` already gated this branch; the rest is
	// defensive against future allowlist drift.
	throw new Error(`insert_text refused to write into <${el.localName}>`);
}

function setNativeValue(el: Element, prototype: object, value: string): void {
	const descriptor = Object.getOwnPropertyDescriptor(prototype, 'value');
	const setter = descriptor?.set;
	if (setter) {
		setter.call(el, value);
		return;
	}
	// Last-resort fallback for runtimes that don't expose the prototype
	// setter (jsdom historically did; modern engines all do).
	(el as unknown as { value: string }).value = value;
}

function dispatchInputEvent(el: Element): void {
	const target = el as HTMLElement;
	const event = new (target.ownerDocument?.defaultView?.InputEvent ?? InputEvent)('input', {
		bubbles: true,
		cancelable: true,
	});
	target.dispatchEvent(event);
}

function violation(message: string): NativeResponse {
	return { kind: 'Error', code: SAFETY_VIOLATION, data: message } as NativeResponse;
}

function describe(err: unknown): string {
	return err instanceof Error ? err.message : String(err);
}
