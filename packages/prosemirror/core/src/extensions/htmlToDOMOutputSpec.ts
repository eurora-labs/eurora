export function htmlToDOMOutputSpec(
	el: HTMLElement,
	collected: any[] = [],
	recursed: { holeFound: boolean } = { holeFound: false }
): any[] {
	collected.push(el.tagName.toLowerCase());
	const attrs = {} as { [attr: string]: string };
	// console.log(el.outerHTML)
	// console.log(Array.from(el.attributes))
	let foundHole = false;
	Array.from(el.attributes).forEach((attr) => {
		if (attr.name === 'data-hole') {
			if (recursed.holeFound) {
				throw Error(
					'@my-org/core: Duplicate holes provided! Remember to only set "data-hole" attribute once!'
				);
			}
			recursed.holeFound = true;
			foundHole = true;
		} else {
			attrs[attr.name] = attr.value;
		}
	});
	if (Object.keys(attrs).length > 0) {
		collected.push(attrs);
	}
	if (foundHole) {
		collected.push(0);
	}
	if (el.children.length > 0) {
		collected = Array.from(el.children).reduce(
			(acc, cur) => [...acc, htmlToDOMOutputSpec(cur as HTMLElement, [], recursed)],
			collected
		);
	}
	return collected;
}
