import { isVisible, writableFieldKind } from './element-filter';
import { buildSelectorPath } from './selector-path';
import type { FormInput, FormInputsList, ListFormInputsArgs } from '../../bindings';
import type { BrowserObj } from '../watchers/watcher';

const DEFAULT_LIMIT = 100;
const HARD_LIMIT = 500;
const FIELD_SELECTORS = 'input, textarea, [contenteditable]';

/**
 * Inventory of *text-typed* editable fields with labels and current
 * values. Password, file, hidden, submit, checkbox, radio, and image
 * inputs are excluded by [`writableFieldKind`]. Disabled, read-only,
 * and invisible fields are excluded too — the model can't usefully
 * write into them anyway.
 *
 * `total` is the pre-`limit` count of allowlisted fields, so the LLM
 * can opt into a higher cap when working with long forms.
 */
export async function handleListFormInputs(obj: BrowserObj): Promise<FormInputsList> {
	const args = parseArgs(obj);
	const root = resolveRoot(args.root_selector);
	if (!root) {
		throw new Error(`root_selector "${args.root_selector ?? '<body>'}" matched no element`);
	}

	const limit = clampLimit(args.limit ?? DEFAULT_LIMIT);
	const inputs: FormInput[] = [];
	let total = 0;

	for (const candidate of Array.from(root.querySelectorAll<HTMLElement>(FIELD_SELECTORS))) {
		const kind = writableFieldKind(candidate);
		if (!kind) {
			continue;
		}
		if (!isVisible(candidate)) {
			continue;
		}
		total += 1;
		if (inputs.length >= limit) {
			continue;
		}

		inputs.push({
			field_id: safeSelectorPath(candidate),
			label: resolveLabel(candidate),
			kind,
			value: readValue(candidate),
			placeholder: readPlaceholder(candidate),
			required: readRequired(candidate),
		});
	}

	return { inputs, total };
}

function parseArgs(obj: BrowserObj): ListFormInputsArgs {
	const rootSelector = obj['root_selector'];
	const rawLimit = obj['limit'];
	return {
		root_selector:
			typeof rootSelector === 'string' && rootSelector.length > 0 ? rootSelector : null,
		limit: typeof rawLimit === 'number' && Number.isFinite(rawLimit) ? rawLimit : undefined,
	};
}

function resolveRoot(selector: string | null): Element | null {
	if (!selector) {
		return document.body;
	}
	try {
		return document.querySelector(selector);
	} catch {
		return null;
	}
}

function clampLimit(raw: number): number {
	if (!Number.isInteger(raw) || raw <= 0) {
		return DEFAULT_LIMIT;
	}
	return Math.min(raw, HARD_LIMIT);
}

function resolveLabel(el: HTMLElement): string | null {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		if (el.labels && el.labels.length > 0) {
			const parts: string[] = [];
			for (const label of Array.from(el.labels)) {
				const text = (label.textContent ?? '').replace(/\s+/g, ' ').trim();
				if (text) {
					parts.push(text);
				}
			}
			if (parts.length > 0) {
				return parts.join(' ');
			}
		}
	}

	const wrapping = el.closest('label');
	if (wrapping) {
		const text = (wrapping.textContent ?? '').replace(/\s+/g, ' ').trim();
		if (text) {
			return text;
		}
	}

	const labelledBy = el.getAttribute('aria-labelledby');
	if (labelledBy) {
		const text = joinIdRefs(el.ownerDocument ?? document, labelledBy);
		if (text) {
			return text;
		}
	}

	const ariaLabel = el.getAttribute('aria-label')?.trim();
	if (ariaLabel) {
		return ariaLabel;
	}

	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		const placeholder = el.placeholder?.trim();
		if (placeholder) {
			return placeholder;
		}
	}

	return null;
}

function joinIdRefs(doc: Document, refs: string): string | null {
	const parts: string[] = [];
	for (const id of refs.split(/\s+/)) {
		if (!id) continue;
		const target = doc.getElementById(id);
		const text = target?.textContent?.replace(/\s+/g, ' ').trim();
		if (text) {
			parts.push(text);
		}
	}
	return parts.length > 0 ? parts.join(' ') : null;
}

function readValue(el: HTMLElement): string {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		return el.value;
	}
	return el.textContent ?? '';
}

function readPlaceholder(el: HTMLElement): string | null {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		const value = el.placeholder?.trim();
		return value ? value : null;
	}
	const ariaPlaceholder = el.getAttribute('aria-placeholder')?.trim();
	return ariaPlaceholder ? ariaPlaceholder : null;
}

function readRequired(el: HTMLElement): boolean {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		if (el.required) {
			return true;
		}
	}
	return el.getAttribute('aria-required') === 'true';
}

function safeSelectorPath(el: Element): string {
	try {
		return buildSelectorPath(el);
	} catch {
		return '';
	}
}
