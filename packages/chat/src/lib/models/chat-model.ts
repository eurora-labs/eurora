export interface ChatModel {
	id: string;
	name: string;
	provider: string;
}

export const DEFAULT_MODELS: ChatModel[] = [
	{ id: 'glm-5.1', name: 'GLM-5.1: Multimodal', provider: 'zai' },
];
