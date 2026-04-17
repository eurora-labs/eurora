import type { ContextChip } from '$lib/bindings/bindings.js';
import type { ChatService, Suggestion } from '@eurora/chat';

const NO_ACTIVE_PAGE_RESPONSE =
	"I don't have an active page. Please first click on any website in your browser to ask questions about it";

type KnownDomainBuilder = (params: {
	domain: string;
	send: (text: string) => void;
}) => Suggestion[];

const KNOWN_DOMAIN_SUGGESTIONS: Record<string, KnownDomainBuilder> = {
	'x.com': ({ send }) => [
		{
			label: 'Is this true?',
			onSelect: () => send('Is this true?'),
		},
		{
			label: 'What does he mean by this?',
			onSelect: () => send('What does he mean by this?'),
		},
		{
			label: 'Summarize this discussion',
			onSelect: () => send('Summarize this discussion'),
		},
	],
	'youtube.com': ({ send }) => [
		{
			label: 'Summarize this video',
			onSelect: () => send('Summarize this video'),
		},
		{
			label: 'What does the presenter mean here?',
			onSelect: () => send('What does the presenter mean here?'),
		},
		{
			label: 'Explain the video frame',
			onSelect: () => send('Explain the video frame'),
		},
	],
	'docs.google.com': ({ send }) => [
		{
			label: 'Summarize this document',
			onSelect: () => send('Summarize this document'),
		},
		{
			label: 'Provide feedback',
			onSelect: () => send('Provide feedback'),
		},
		{
			label: 'Translate this',
			onSelect: () => send('Translate this'),
		},
	],
};

function matchKnownDomain(domain: string): string | undefined {
	return Object.keys(KNOWN_DOMAIN_SUGGESTIONS).find(
		(known) => domain === known || domain.endsWith(`.${known}`),
	);
}

function genericDomainSuggestions(domain: string, send: (text: string) => void): Suggestion[] {
	return [
		{ label: `Summarize ${domain}`, onSelect: () => send(`Summarize ${domain}`) },
		{
			label: `Explain what I highlighted on ${domain}`,
			onSelect: () => send(`Explain what I highlighted on ${domain}`),
		},
		{ label: `Translate ${domain}`, onSelect: () => send(`Translate ${domain}`) },
	];
}

function noContextSuggestion(chatService: ChatService): Suggestion {
	const label = 'Explain the browser page';
	return {
		label,
		onSelect: () => chatService.addLocalExchange(label, NO_ACTIVE_PAGE_RESPONSE),
	};
}

export function buildSuggestions(params: {
	chips: ContextChip[];
	chatService: ChatService;
	send: (text: string) => void;
}): Suggestion[] {
	const { chips, chatService, send } = params;

	const domain = chips.find((c) => c.domain)?.domain;
	if (!domain) return [noContextSuggestion(chatService)];

	const knownKey = matchKnownDomain(domain);
	if (knownKey) return KNOWN_DOMAIN_SUGGESTIONS[knownKey]({ domain, send });

	return genericDomainSuggestions(domain, send);
}
