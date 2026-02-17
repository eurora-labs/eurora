import type { SveltePMExtension } from '@eurora/prosemirror-core/index';

export type ExtensionCreator = () => SveltePMExtension;

export class ExtensionFactory {
	private registry: Map<string, ExtensionCreator> = new Map();

	register(id: string, creator: ExtensionCreator): void {
		if (this.registry.has(id)) {
			console.warn(`Extension with ID ${id} is already registered. It will be overwritten.`);
		}
		this.registry.set(id, creator);
	}

	getExtension(id: string): SveltePMExtension {
		const creator = this.registry.get(id);
		if (!creator) {
			throw new Error(`Extension with ID ${id} not found`);
		}
		return creator();
	}

	getExtensions(): SveltePMExtension[] {
		return Array.from(this.registry.values()).map((creator) => creator());
	}

	hasExtension(id: string): boolean {
		return this.registry.has(id);
	}

	getExtensionIds(): string[] {
		return Array.from(this.registry.keys());
	}

	unregister(id: string): boolean {
		return this.registry.delete(id);
	}
}

export const extensionFactory = new ExtensionFactory();

export type { SveltePMExtension };
