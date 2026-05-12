import { startBridgeClient } from '$lib/bridge/client';
import { dispatchRequest } from '$lib/bridge/handlers';
import * as log from '$lib/util/log';

declare global {
	var onDocumentOpened: ((event: Office.AddinCommands.Event) => void) | undefined;
}

Office.onReady((info) => {
	if (info.host !== Office.HostType.Word) {
		log.warn('unsupported host, runtime will idle:', info.host);
		return;
	}
	startBridgeClient({ dispatch: dispatchRequest });
});

function onDocumentOpened(event: Office.AddinCommands.Event): void {
	log.info('LaunchEvent fired');
	event.completed();
}

globalThis.onDocumentOpened = onDocumentOpened;
