import type { NativeTwitterTweet } from '../../../../shared/content/bindings';

export interface TweetPageData {
	tweet: NativeTwitterTweet | null;
	replies: NativeTwitterTweet[];
}

export interface ProfilePageData {
	username: string;
	tweets: NativeTwitterTweet[];
}

export interface TimelineData {
	tweets: NativeTwitterTweet[];
}

export interface SearchData {
	query: string;
	tweets: NativeTwitterTweet[];
}

export interface NotificationsData {
	tweets: NativeTwitterTweet[];
}

export interface UnsupportedPageData {
	url: string;
}

export type ParseResult =
	| { page: 'tweet'; data: TweetPageData }
	| { page: 'profile'; data: ProfilePageData }
	| { page: 'home'; data: TimelineData }
	| { page: 'search'; data: SearchData }
	| { page: 'notifications'; data: NotificationsData }
	| { page: 'unsupported'; data: UnsupportedPageData };
