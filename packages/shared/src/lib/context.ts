import { setContext, getContext as svelteGetContext } from 'svelte';

export class InjectionToken<_T> {
	private readonly _desc: string;
	private readonly _symbol: symbol;

	constructor(desc: string) {
		this._desc = desc;
		this._symbol = Symbol(desc);
	}

	get description(): string {
		return this._desc;
	}

	toString(): string {
		return `InjectionToken(${this._desc})`;
	}

	get _key(): symbol {
		return this._symbol;
	}
}

export function provide<T>(token: InjectionToken<T>, value: T): void {
	setContext(token._key, value);
}

export function provideAll(entries: [InjectionToken<any>, any][]) {
	for (const [token, value] of entries) {
		provide(token, value);
	}
}

export function inject<T>(token: InjectionToken<T>): T {
	const value = svelteGetContext<T>(token._key);
	if (value === undefined) {
		throw new Error(`No provider found for ${token.toString()}`);
	}
	return value;
}

export function injectOptional<T>(token: InjectionToken<T>, defaultValue: T): T {
	const value = svelteGetContext<T>(token._key);
	return value !== undefined ? value : defaultValue;
}
