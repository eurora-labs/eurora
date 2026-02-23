<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';
	import type {
		HttpMethod,
		SchemaParameter,
		SchemaProperty,
	} from './schema-display-context.svelte.js';
	import { cn } from '$lib/utils.js';
	import { setSchemaDisplay } from './schema-display-context.svelte.js';

	let {
		class: className,
		method,
		path,
		description,
		parameters,
		requestBody,
		responseBody,
		children,
		...restProps
	}: HTMLAttributes<HTMLDivElement> & {
		method: HttpMethod;
		path: string;
		description?: string;
		parameters?: SchemaParameter[];
		requestBody?: SchemaProperty[];
		responseBody?: SchemaProperty[];
		children?: Snippet;
	} = $props();

	setSchemaDisplay({
		method: () => method,
		path: () => path,
		description: () => description,
		parameters: () => parameters,
		requestBody: () => requestBody,
		responseBody: () => responseBody,
	});
</script>

<div
	data-slot="schema-display"
	class={cn('overflow-hidden rounded-lg border bg-background', className)}
	{...restProps}
>
	{@render children?.()}
</div>
