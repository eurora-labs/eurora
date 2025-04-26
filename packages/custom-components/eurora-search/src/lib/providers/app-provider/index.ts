import type { SearchProvider } from '../../types/search-provider';
import { detectOperatingSystem, OperatingSystem } from '../../utils/platform';
import { WindowsAppProvider } from './windows';
import { MacOsAppProvider } from './macos';
import { LinuxAppProvider } from './linux';

/**
 * Creates an app provider based on the current operating system
 */
export function createAppProvider(): SearchProvider {
	const os = detectOperatingSystem();

	switch (os) {
		case OperatingSystem.WINDOWS:
			return new WindowsAppProvider();
		case OperatingSystem.MACOS:
			return new MacOsAppProvider();
		case OperatingSystem.LINUX:
			return new LinuxAppProvider();
		default:
			throw new Error(`Unsupported operating system: ${os}`);
	}
}
