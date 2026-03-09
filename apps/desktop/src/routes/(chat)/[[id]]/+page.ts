export function load({ params }) {
	return {
		threadId: params.id ?? null,
	};
}
