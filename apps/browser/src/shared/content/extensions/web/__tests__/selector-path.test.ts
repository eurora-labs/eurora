import { buildSelectorPath } from '../selector-path';
import { describe, it, expect, beforeEach } from 'vitest';

function setBody(html: string): void {
	document.body.innerHTML = html;
}

function selectorRoundTrips(selector: string, target: Element): boolean {
	const matches = document.querySelectorAll(selector);
	return matches.length === 1 && matches[0] === target;
}

describe('buildSelectorPath', () => {
	beforeEach(() => {
		setBody('');
	});

	it('uses #id when the id is a simple CSS identifier and unique', () => {
		setBody('<form><input id="login-email" type="email"></form>');
		const input = document.querySelector('input')!;
		const path = buildSelectorPath(input);
		expect(path).toBe('#login-email');
		expect(selectorRoundTrips(path, input)).toBe(true);
	});

	it('falls back when id is non-unique', () => {
		setBody('<div id="dup"></div><div id="dup"></div>');
		const second = document.querySelectorAll('#dup')[1]!;
		const path = buildSelectorPath(second);
		expect(path.startsWith('#')).toBe(false);
		expect(selectorRoundTrips(path, second)).toBe(true);
	});

	it('prefers data-testid over a positional chain', () => {
		setBody(`
			<main>
				<section>
					<button data-testid="submit-btn">Submit</button>
					<button>Cancel</button>
				</section>
			</main>
		`);
		const button = document.querySelector('[data-testid="submit-btn"]')!;
		const path = buildSelectorPath(button);
		expect(path).toContain('data-testid="submit-btn"');
		expect(selectorRoundTrips(path, button)).toBe(true);
	});

	it('roots the chain at the nearest semantic landmark when no id/attr is stable', () => {
		setBody(`
			<main>
				<section>
					<p>One</p>
					<p>Two</p>
					<p>Three</p>
				</section>
			</main>
		`);
		const second = document.querySelectorAll('p')[1]!;
		const path = buildSelectorPath(second);
		expect(path.startsWith('main')).toBe(true);
		expect(selectorRoundTrips(path, second)).toBe(true);
	});

	it('falls back to an absolute chain when no landmark is available', () => {
		setBody('<div><span></span><span></span></div>');
		const second = document.querySelectorAll('span')[1]!;
		const path = buildSelectorPath(second);
		expect(selectorRoundTrips(path, second)).toBe(true);
	});

	it('produces selectors that round-trip across deeply nested structures', () => {
		setBody(`
			<main>
				<article>
					<header>
						<h1>Title</h1>
					</header>
					<section>
						<ul>
							<li><a href="#">One</a></li>
							<li><a href="#">Two</a></li>
							<li><a href="#">Three</a></li>
						</ul>
					</section>
				</article>
			</main>
		`);
		const anchors = document.querySelectorAll('a');
		for (const a of Array.from(anchors)) {
			const path = buildSelectorPath(a);
			expect(selectorRoundTrips(path, a)).toBe(true);
		}
	});
});
