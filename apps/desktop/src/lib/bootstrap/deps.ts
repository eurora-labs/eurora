import { APPEARANCE_SERVICE, AppearanceService } from '$lib/services/appearance-service.svelte.js';
import { GENERAL_SERVICE, GeneralService } from '$lib/services/general-service.svelte.js';
import { TELEMETRY_SERVICE, TelemetryService } from '$lib/services/telemetry-service.svelte.js';
import { ThreadService } from '$lib/services/thread-service.svelte.js';
import { TIMELINE_SERVICE, TimelineService } from '$lib/services/timeline-service.svelte.js';
import { USER_SERVICE, UserService } from '$lib/services/user-service.svelte.js';
import { ChatService, CHAT_SERVICE } from '@eurora/chat/services/chat/chat-service.svelte';
import { THREAD_SERVICE } from '@eurora/chat/services/thread/thread-service';
import { provideAll } from '@eurora/shared/context';

export function initDependencies() {
	const threadClient = new ThreadService();
	const appearance = new AppearanceService();
	const telemetry = new TelemetryService();
	return provideAll([
		[TELEMETRY_SERVICE, telemetry],
		[THREAD_SERVICE, threadClient],
		[USER_SERVICE, new UserService(telemetry)],
		[CHAT_SERVICE, new ChatService(threadClient)],
		[APPEARANCE_SERVICE, appearance],
		[GENERAL_SERVICE, new GeneralService()],
		[TIMELINE_SERVICE, new TimelineService(appearance)],
	]);
}
