import { HomePageParser } from './pages/home';
import { NotificationsPageParser } from './pages/notifications';
import { ProfilePageParser } from './pages/profile';
import { SearchPageParser } from './pages/search';
import { TweetPageParser } from './pages/tweet';
import { UnsupportedPageParser } from './pages/unsupported';
import type { BasePageParser } from './base';
import type { ParseResult } from './types';

interface Route {
	match: (pathname: string) => boolean;
	parser: BasePageParser;
}

const UNSUPPORTED_PREFIXES = ['/settings', '/i/', '/messages', '/compose', '/lists', '/bookmarks'];

const routes: Route[] = [
	{
		match: (p) => p === '/' || p === '/home',
		parser: new HomePageParser(),
	},
	{
		match: (p) => p.startsWith('/search'),
		parser: new SearchPageParser(),
	},
	{
		match: (p) => p.startsWith('/notifications'),
		parser: new NotificationsPageParser(),
	},
	{
		match: (p) => UNSUPPORTED_PREFIXES.some((prefix) => p.startsWith(prefix)),
		parser: new UnsupportedPageParser(),
	},
	{
		match: (p) => /^\/[^/]+\/status\/\d+/.test(p),
		parser: new TweetPageParser(),
	},
	{
		match: (p) => /^\/[^/]+\/?$/.test(p) && p !== '/',
		parser: new ProfilePageParser(),
	},
];

const fallback = new UnsupportedPageParser();

export class TwitterParser {
	async parse(doc: Document): Promise<ParseResult> {
		const pathname = window.location.pathname;
		const route = routes.find((r) => r.match(pathname));
		const parser = route?.parser ?? fallback;
		return await parser.parse(doc);
	}
}

export type { ParseResult } from './types';
export type {
	TweetPageData,
	ProfilePageData,
	TimelineData,
	SearchData,
	NotificationsData,
	UnsupportedPageData,
} from './types';
