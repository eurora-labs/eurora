export interface RemoveMessage {
	type: 'remove';
	id: string;
	name: string | null;
	additionalKwargs: string | null;
	responseMetadata: string | null;
}
