type UsageInput = {
	input?: number;
	output?: number;
	reasoningTokens?: number;
	cacheReads?: number;
};

let getUsageFn:
	| ((opts: { modelId: string; usage: UsageInput }) => { costUSD?: { totalUSD?: number } })
	| undefined;

try {
	// @ts-ignore
	const tokenlens = await import('tokenlens');
	getUsageFn = tokenlens.getUsage;
} catch {
	// tokenlens not available
}

export function computeCost(modelId: string, usage: UsageInput): number | undefined {
	if (!getUsageFn) return undefined;
	try {
		return getUsageFn({ modelId, usage })?.costUSD?.totalUSD;
	} catch {
		return undefined;
	}
}
