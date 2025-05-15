/**
 * Transforms HTML node and a DOMOutputSpec into a ParseRule for parseDOM method
 *
 * @example
 * ['figure', { id }, ['img', { src, alt }], ['figcaption', { caption }, 0]]
 * ->
 * {
 *   tag: 'figure',
 *   getAttrs: (dom: HTMLElement | string) => {
 *     if (dom instanceof HTMLElement) {
 *       return {
 *         id: dom.getAttribute('id'),
 *         src: dom.querySelector('figure > img').getAttribute('src'),
 *         alt: dom.querySelector('figure > img').getAttribute('alt'),
 *         caption: dom.querySelector('figure > figcaption').getAttribute('caption'),
 *       }
 *     }
 *     return null
 *   }
 * }
 *
 * @param dom
 * @param spec
 * @param customHandler
 */
export function getAttrsWithOutputSpec(
	spec: readonly any[],
	dom: HTMLElement,
	output: any,
	customHandler?: (el: HTMLElement, attr: string) => any,
) {
	let attrs = {} as { [attr: string]: any };
	if (!output.tag) {
		// TODO should use default values & DOMOutputSpec union to derive what's defined
		output.tag = dom.tagName.toLowerCase();
		if (dom.classList.length > 1) {
			output.tag += Array.from(dom.classList.values()).join('.');
		}
	}
	let idx = 0;
	while (spec.length > idx) {
		const val = spec[idx];
		// console.log('val', val)
		if (typeof val === 'string') {
			output.selector.push(val);
			idx += 1;
		} else if (Array.isArray(val)) {
			// move inwards
			attrs = { ...attrs, ...getAttrsWithOutputSpec(val, dom, output, customHandler) };
			output.selector.pop();
			idx += 1;
		} else if (typeof val === 'object' && val !== null) {
			// extract attributes
			// if (customHandler()) attr = customHandler()
			const s = output.selector.join(' > ') as string;
			const el = dom.querySelector(s);
			Array.from(el?.attributes || []).forEach(attr => {
				const { name, value } = attr;
				if (name in val) {
					if (value === 'true') {
						attrs[name] = true;
					} else if (value === 'false') {
						attrs[name] = false;
					} else if (/^\d+$/.test(value)) {
						attrs[name] = parseInt(value);
					} else if (/^[+-]?\d+(\.\d+)?$/.test(value)) {
						attrs[name] = parseFloat(value);
					} else {
						attrs[name] = value;
					}
				} else {
					console.warn(
						`Found attribute (${name}) which wasnt defined in attribute object:`,
						val,
					);
				}
			});
			idx += 1;
		} else {
			// unknown, probably a hole?
			idx += 1;
		}
	}
	return attrs;
}
