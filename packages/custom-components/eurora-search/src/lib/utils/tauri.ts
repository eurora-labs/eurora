import { invoke } from '@tauri-apps/api/core';
import type { AppInfo } from '../types/app-info';
import type { Document } from '../types/document';
/**
 * Tauri command wrappers for platform-specific app search
 */

/**
 * Search for applications on Windows
 */
export async function searchWindowsApps(query: string): Promise<AppInfo[]> {
	try {
		return await invoke<AppInfo[]>('search_windows_apps', { query });
	} catch (error) {
		console.error('Error searching Windows apps:', error);
		return [];
	}
}

/**
 * Search for applications on macOS
 */
export async function searchMacOsApps(query: string): Promise<AppInfo[]> {
	try {
		return await invoke<AppInfo[]>('search_macos_apps', { query });
	} catch (error) {
		console.error('Error searching macOS apps:', error);
		return [];
	}
}

/**
 * Search for applications on Linux
 */
export async function searchLinuxApps(query: string): Promise<AppInfo[]> {
	try {
		console.log('==== Searching Linux apps with query:', query);
		const documents = await invoke<Document[]>('search_linux_apps', { query });
		console.log('==== Received documents:', documents.length);

		// Convert Document[] to AppInfo[]
		const apps: AppInfo[] = documents.map((doc) => ({
			name: doc.title,
			icon: doc.icon,
			path: ''
		}));

		return apps;
	} catch (error) {
		console.error('Error searching Linux apps:', error);
		return [];
	}
}

/**
 * Launch an application by path
 */
export async function launchApplication(path: string): Promise<void> {
	try {
		await invoke<void>('launch_application', { path });
	} catch (error) {
		console.error('Error launching application:', error);
		throw error;
	}
}
