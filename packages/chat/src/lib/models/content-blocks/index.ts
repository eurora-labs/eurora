export interface TextContentBlock {
	type: 'text';
	id: string | null;
	text: string;
	annotations: Annotation[];
	index: BlockIndex | null;
	extras: string | null;
}

export interface ReasoningContentBlock {
	type: 'reasoning';
	id: string | null;
	reasoning: string | null;
	index: BlockIndex | null;
	extras: string | null;
}

export interface ImageContentBlock {
	type: 'image';
	id: string | null;
	fileId: string | null;
	mimeType: string | null;
	index: BlockIndex | null;
	url: string | null;
	base64: string | null;
	extras: string | null;
}

export interface VideoContentBlock {
	type: 'video';
	id: string | null;
	fileId: string | null;
	mimeType: string | null;
	index: BlockIndex | null;
	url: string | null;
	base64: string | null;
	extras: string | null;
}

export interface AudioContentBlock {
	type: 'audio';
	id: string | null;
	fileId: string | null;
	mimeType: string | null;
	index: BlockIndex | null;
	url: string | null;
	base64: string | null;
	extras: string | null;
}

export interface PlainTextContentBlock {
	type: 'plainText';
	id: string | null;
	fileId: string | null;
	mimeType: string;
	index: BlockIndex | null;
	url: string | null;
	base64: string | null;
	text: string | null;
	title: string | null;
	context: string | null;
	extras: string | null;
}

export interface FileContentBlock {
	type: 'file';
	id: string | null;
	fileId: string | null;
	mimeType: string | null;
	index: BlockIndex | null;
	url: string | null;
	base64: string | null;
	extras: string | null;
}

export interface NonStandardContentBlock {
	type: 'nonStandard';
	id: string | null;
	value: string;
	index: BlockIndex | null;
}

export interface ToolCallBlock {
	type: 'toolCall';
	id: string | null;
	name: string;
	args: string;
	index: BlockIndex | null;
	extras: string | null;
}

export interface ToolCallChunkBlock {
	type: 'toolCallChunk';
	id: string | null;
	name: string | null;
	args: string | null;
	index: BlockIndex | null;
	extras: string | null;
}

export interface InvalidToolCallBlock {
	type: 'invalidToolCall';
	id: string | null;
	name: string | null;
	args: string | null;
	error: string | null;
	index: BlockIndex | null;
	extras: string | null;
}

export interface ServerToolCall {
	type: 'serverToolCall';
	id: string;
	name: string;
	args: string;
	index: BlockIndex | null;
	extras: string | null;
}

export interface ServerToolCallChunk {
	type: 'serverToolCallChunk';
	id: string | null;
	name: string | null;
	args: string | null;
	index: BlockIndex | null;
	extras: string | null;
}

export interface ServerToolResult {
	type: 'serverToolResult';
	id: string | null;
	toolCallId: string;
	status: number;
	output: string | null;
	index: BlockIndex | null;
	extras: string | null;
}

export type ContentBlock =
	| TextContentBlock
	| ReasoningContentBlock
	| ImageContentBlock
	| VideoContentBlock
	| AudioContentBlock
	| PlainTextContentBlock
	| FileContentBlock
	| NonStandardContentBlock
	| ToolCallBlock
	| ToolCallChunkBlock
	| InvalidToolCallBlock
	| ServerToolCall
	| ServerToolCallChunk
	| ServerToolResult;

export type BlockIndex = { type: 'int'; value: bigint } | { type: 'str'; value: string };

export interface Citation {
	id: string | null;
	url: string | null;
	title: string | null;
	startIndex: bigint | null;
	endIndex: bigint | null;
	citedText: string | null;
	extras: string | null;
}

export interface NonStandardAnnotation {
	id: string | null;
	value: string;
}

export type Annotation =
	| { type: 'citation'; value: Citation }
	| { type: 'nonStandard'; value: NonStandardAnnotation };
