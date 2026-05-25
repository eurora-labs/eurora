import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import { isVisible, writableFieldKind } from '../../extensions/web/element-filter';
import { buildSelectorPath } from '../../extensions/web/selector-path';
import type { Tool } from '../types';

const DEFAULT_LIMIT = 100;
const HARD_LIMIT = 500;
const FIELD_SELECTORS = 'input, textarea, [contenteditable]';

const Args = z
	.object({
		root_selector: z.string().min(1).optional(),
		limit: z.number().int().positive().optional(),
	})
	.strict();

const FormInput = z.object({
	field_id: z.string(),
	label: z.string().nullable(),
	kind: z.enum(['text', 'search', 'email', 'url', 'tel', 'number', 'textarea', 'content_editable']),
	value: z.string(),
	placeholder: z.string().nullable(),
	required: z.boolean(),
});

const Out = z.object({
	inputs: z.array(FormInput),
	total: z.number().int().nonnegative(),
});

type Result = z.infer<typeof Out>;
type FormInputT = z.infer<typeof FormInput>;

function resolveRoot(selector: string | undefined): Element | null {
	if (!selector) return document.body;
	try {
		return document.querySelector(selector);
	} catch {
		return null;
	}
}

function joinIdRefs(doc: Document, refs: string): string | null {
	const parts: string[] = [];
	for (const id of refs.split(/\s+/)) {
		if (!id) continue;
		const target = doc.getElementById(id);
		const text = target?.textContent?.replace(/\s+/g, ' ').trim();
		if (text) parts.push(text);
	}
	return parts.length > 0 ? parts.join(' ') : null;
}

function resolveLabel(el: HTMLElement): string | null {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		if (el.labels && el.labels.length > 0) {
			const parts: string[] = [];
			for (const label of Array.from(el.labels)) {
				const text = (label.textContent ?? '').replace(/\s+/g, ' ').trim();
				if (text) parts.push(text);
			}
			if (parts.length > 0) return parts.join(' ');
		}
	}

	const wrapping = el.closest('label');
	if (wrapping) {
		const text = (wrapping.textContent ?? '').replace(/\s+/g, ' ').trim();
		if (text) return text;
	}

	const labelledBy = el.getAttribute('aria-labelledby');
	if (labelledBy) {
		const text = joinIdRefs(el.ownerDocument ?? document, labelledBy);
		if (text) return text;
	}

	const ariaLabel = el.getAttribute('aria-label')?.trim();
	if (ariaLabel) return ariaLabel;

	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) {
		const placeholder = el.placeholder?.trim();
		if (placeholder) return placeholder;
	}
	return null;
}

function readValue(el: HTMLElement): string {
	if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) return el.value;
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
		if (el.required) return true;
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

export async function executeListFormInputs(args: z.infer<typeof Args>): Promise<Result> {
	const root = resolveRoot(args.root_selector);
	if (!root) {
		throw new Error(`root_selector "${args.root_selector ?? '<body>'}" matched no element`);
	}

	const limit = Math.min(args.limit ?? DEFAULT_LIMIT, HARD_LIMIT);
	const inputs: FormInputT[] = [];
	let total = 0;

	for (const candidate of Array.from(root.querySelectorAll<HTMLElement>(FIELD_SELECTORS))) {
		const kind = writableFieldKind(candidate);
		if (!kind) continue;
		if (!isVisible(candidate)) continue;
		total += 1;
		if (inputs.length >= limit) continue;
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

export const listFormInputs: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'web_list_form_inputs',
		description:
			"Inventory of text-typed editable fields with labels and current values. Password, file, hidden, submit, checkbox, radio, and image inputs are excluded by the safety contract. Disabled, read-only, and invisible fields are excluded too. `total` is the pre-`limit` count.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 3_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return executeListFormInputs(args);
	},
};
