export interface ThreadSearchResult {
	id: string;
	title: string;
	rank: number;
}

export interface MessageSearchResult {
	id: string;
	threadId: string;
	messageType: string;
	snippet: string;
	rank: number;
}
