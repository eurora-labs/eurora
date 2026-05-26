import { getPageKind, type PageKind } from './_lib';
import { getCurrentFrame } from './get_current_frame';
import { getCurrentTimestamp } from './get_current_timestamp';
import { getPageContext } from './get_page_context';
import { getTranscript } from './get_transcript';
import { getVideoMetadata } from './get_video_metadata';
import { listCaptions } from './list_captions';
import { listChapters } from './list_chapters';
import { listRecommendations } from './list_recommendations';
import { listSearchResults } from './list_search_results';
import { seekTo } from './seek_to';
import type { Tool } from '../types';
import type { z } from 'zod';

export {
	getCurrentFrame,
	getCurrentTimestamp,
	getPageContext,
	getTranscript,
	getVideoMetadata,
	listCaptions,
	listChapters,
	listRecommendations,
	listSearchResults,
	seekTo,
};
export {
	getPageKind,
	readPlayerTime,
	resolveChannelHandle,
	resolveChannelId,
	resolvePlaylistId,
	resolveSearchQuery,
	resolveShortsVideoId,
	resolveWatchVideoId,
} from './_lib';
export type { PageKind } from './_lib';

type ToolList = readonly Tool<z.ZodTypeAny, unknown>[];

/// Always-on baseline: page context lets the model orient itself even
/// on pages where no other youtube tool is meaningful.
const BASE: ToolList = [getPageContext] as const;

const WATCH: ToolList = [
	getPageContext,
	getCurrentTimestamp,
	getCurrentFrame,
	getTranscript,
	getVideoMetadata,
	listChapters,
	listCaptions,
	listRecommendations,
	seekTo,
] as const;

const SHORTS: ToolList = [getPageContext, getCurrentTimestamp, getCurrentFrame] as const;

const SEARCH: ToolList = [getPageContext, listSearchResults] as const;

/// Pick the youtube tool slice appropriate for the current page kind.
/// Re-evaluated by the watcher per `LIST_TOOLS` call so SPA navigation
/// (e.g. clicking from `/watch` into `/results`) updates the surface
/// without a content-script reload.
export function resolveYoutubeTools(pathname: string = window.location.pathname): ToolList {
	const kind: PageKind = getPageKind(pathname);
	switch (kind) {
		case 'watch':
			return WATCH;
		case 'shorts':
			return SHORTS;
		case 'search':
			return SEARCH;
		case 'channel':
		case 'playlist':
		case 'home':
		case 'unsupported':
			return BASE;
	}
}
