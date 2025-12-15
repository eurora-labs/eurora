export const base = {
	manifest_version: 3,
	name: 'Eurora',
	version: '0.0.0',
	minimum_chrome_version: '102.0',
	action: { default_popup: 'popup.html' },
	permissions: [
		'nativeMessaging',
		'tabs',
		'storage',
		'scripting',
		'declarativeNetRequestWithHostAccess',
		'webRequest',
		'webNavigation',
	],
	icons: {
		16: 'icon-16x16.png',
		32: 'icon-32x32.png',
		48: 'icon-48x48.png',
		128: 'icon-128x128.png',
	},
	host_permissions: ['<all_urls>'],
	background: { service_worker: 'assets/background.js', type: 'module' },
	storage: {
		managed_schema: 'preferences_schema.json',
	},
	// Content scripts are managed programmatically by the background script
	// using browser.scripting.executeScript, not declared here
	web_accessible_resources: [
		{
			resources: [
				'assets/*',
				'content/web/viewer.html',
				'pdfjs/viewer/*',
				'http:/*',
				'https:/*',
				'file:/*',
				'chrome-extension:/*',
				'blob:*',
				'data:*',
				'filesystem:/*',
				'drive:*',
				'scripts/content/*',
			],
			matches: ['<all_urls>'],
		},
	],
};
