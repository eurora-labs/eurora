import { getPageKind, type PageKind } from './_lib';
import { fetchTweetImages } from './fetch_tweet_images';
import { getPageContext } from './get_page_context';
import { getTweetThread } from './get_tweet_thread';
import { listTimelineTweets } from './list_timeline_tweets';
import type { Tool } from '../types';
import type { z } from 'zod';

export { fetchTweetImages, getPageContext, getTweetThread, listTimelineTweets };
export { getPageKind, resolveProfileHandle, resolveSearchQuery } from './_lib';
export type { PageKind } from './_lib';

type ToolList = readonly Tool<z.ZodTypeAny, unknown>[];

/// Always-on baseline: page-context lets the model orient itself even
/// on pages where no other twitter tool is meaningful.
const BASE: ToolList = [getPageContext] as const;

const TIMELINE: ToolList = [getPageContext, listTimelineTweets, fetchTweetImages] as const;

const THREAD: ToolList = [getPageContext, getTweetThread, fetchTweetImages] as const;

/// Pick the twitter tool slice appropriate for the current page kind.
/// Re-evaluated by the watcher per `LIST_TOOLS` call so SPA navigation
/// (e.g. clicking from `/home` into `/<handle>/status/<id>`) updates
/// the surface without a content-script reload.
export function resolveTwitterTools(pathname: string = window.location.pathname): ToolList {
	const kind: PageKind = getPageKind(pathname);
	switch (kind) {
		case 'home':
		case 'profile':
		case 'search':
		case 'notifications':
			return TIMELINE;
		case 'tweet':
			return THREAD;
		case 'unsupported':
			return BASE;
	}
}
