import type { Suggestion } from '@eurora/chat';

const DEFAULT_PROMPTS = [
	'What are the latest trends in AI?',
	'How does machine learning work?',
	'Explain quantum computing',
	'Best practices for React development',
];

export function buildSuggestions(send: (text: string) => void): Suggestion[] {
	return DEFAULT_PROMPTS.map((label) => ({ label, onSelect: () => send(label) }));
}
