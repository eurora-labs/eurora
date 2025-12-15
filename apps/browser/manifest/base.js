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
	host_permissions: ['<all_urls>'],
	background: { service_worker: 'assets/background.js', type: 'module' },
	storage: {
		managed_schema: 'preferences_schema.json',
	},
	// Content scripts are managed programmatically by the background script
	// using browser.scripting.executeScript, not declared here
	web_accessible_resources: [
		{
			resources: ['assets/*', 'scripts/content/*'],
			matches: ['<all_urls>'],
		},
	],
};
