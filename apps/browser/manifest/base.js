export const base = {
	manifest_version: 3,
	name: 'Eurora',
	version: '0.0.0',
	minimum_chrome_version: '102.0',
	action: { default_popup: 'popup.html' },
	permissions: ['storage'],
	host_permissions: ['<all_urls>'],
	background: { service_worker: 'assets/background.js', type: 'module' },
	content_scripts: [
		{
			matches: ['<all_urls>'],
			js: ['assets/content.js'],
			run_at: 'document_idle',
		},
	],
	web_accessible_resources: [
		{
			resources: ['assets/*'],
			matches: ['<all_urls>'],
		},
	],
};
