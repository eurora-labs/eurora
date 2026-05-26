import { requireCurrentVideoId, requirePlayer } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z
	.object({
		seconds: z.number().nonnegative(),
	})
	.strict();

const Out = z.object({
	video_id: z.string(),
	previous_time: z.number(),
	new_time: z.number(),
	duration: z.number().nullable(),
	clamped: z.boolean(),
});

type Result = z.infer<typeof Out>;

export async function executeSeekTo(args: z.infer<typeof Args>): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const player = requirePlayer();
	const previousTime = player.currentTime;
	const duration = Number.isFinite(player.duration) ? player.duration : null;
	/// Clamp into `[0, duration]` ourselves so an out-of-range request
	/// surfaces as an explicit `clamped` signal in the response, rather
	/// than HTMLMediaElement silently rejecting the assignment and
	/// leaving the player at its previous position.
	const target = duration !== null ? Math.min(args.seconds, duration) : args.seconds;
	const clamped = target !== args.seconds;
	player.currentTime = target;
	return {
		video_id: videoId,
		previous_time: previousTime,
		new_time: player.currentTime,
		duration,
		clamped,
	};
}

export const seekTo: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_seek_to',
		description:
			'Seek the active YouTube player to `seconds` from the start. The target is clamped to `[0, duration]` when duration is known; the response reports `previous_time`, the effective `new_time`, and a `clamped` flag when the requested position was out of range. Does not change play / pause state.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 1_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(args) {
		return await executeSeekTo(args);
	},
};
