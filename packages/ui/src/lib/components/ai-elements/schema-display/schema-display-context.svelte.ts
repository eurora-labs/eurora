import { getContext, setContext } from 'svelte';

export type HttpMethod = 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export interface SchemaParameter {
	name: string;
	type: string;
	required?: boolean;
	description?: string;
	location?: 'path' | 'query' | 'header';
}

export interface SchemaProperty {
	name: string;
	type: string;
	required?: boolean;
	description?: string;
	properties?: SchemaProperty[];
	items?: SchemaProperty;
}

type Getter<T> = () => T;

export type SchemaDisplayStateProps = {
	method: Getter<HttpMethod>;
	path: Getter<string>;
	description: Getter<string | undefined>;
	parameters: Getter<SchemaParameter[] | undefined>;
	requestBody: Getter<SchemaProperty[] | undefined>;
	responseBody: Getter<SchemaProperty[] | undefined>;
};

class SchemaDisplayState {
	readonly props: SchemaDisplayStateProps;
	method = $derived.by(() => this.props.method());
	path = $derived.by(() => this.props.path());
	description = $derived.by(() => this.props.description());
	parameters = $derived.by(() => this.props.parameters());
	requestBody = $derived.by(() => this.props.requestBody());
	responseBody = $derived.by(() => this.props.responseBody());

	constructor(props: SchemaDisplayStateProps) {
		this.props = props;
	}
}

const SYMBOL_KEY = 'ai-schema-display';

export function setSchemaDisplay(props: SchemaDisplayStateProps): SchemaDisplayState {
	return setContext(Symbol.for(SYMBOL_KEY), new SchemaDisplayState(props));
}

export function useSchemaDisplay(): SchemaDisplayState {
	return getContext(Symbol.for(SYMBOL_KEY));
}
