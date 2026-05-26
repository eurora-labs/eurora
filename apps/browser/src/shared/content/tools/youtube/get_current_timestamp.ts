import { requireCurrentVideoId, requirePlayer } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	video_id: z.string(),
	current_time: z.number(),
	duration: z.number().nullable(),
	playing: z.boolean(),
});

type Result = z.infer<typeof Out>;

export async function executeGetCurrentTimestamp(): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const player = requirePlayer();
	return {
		video_id: videoId,
		current_time: player.currentTime,
		duration: Number.isFinite(player.duration) ? player.duration : null,
		playing: !player.paused,
	};
}

export const getCurrentTimestamp: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_get_current_timestamp',
		description:
			"Return the user's current playback position on the active YouTube watch or shorts page, plus total duration and whether the video is playing. Fails if the page has no player or the player hasn't loaded.",
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 2_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeGetCurrentTimestamp();
	},
};
