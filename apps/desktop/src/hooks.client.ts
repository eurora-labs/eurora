import { info, attachConsole } from '@fltsci/tauri-plugin-tracing';

attachConsole().then(() => {
	info('Javascript Console Attached');
});
