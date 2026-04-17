import type { ContentBlock } from '$lib/models/content-blocks/index.js';
import type { AssetChip } from '$lib/models/messages/asset-chip.js';

export interface HumanMessage {
	type: 'human';
	content: ContentBlock[];
	id: string;
	name: string | null;
	additionalKwargs: string | null;
	responseMetadata: string | null;
	assetChips: AssetChip[];
}
