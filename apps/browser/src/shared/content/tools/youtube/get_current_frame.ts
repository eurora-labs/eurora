import { requireCurrentVideoId, requirePlayer } from './_lib';
import { z } from 'zod';
import { zodToJsonSchema } from 'zod-to-json-schema';
import type { Tool } from '../types';

const Args = z.object({}).strict();

const Out = z.object({
	video_id: z.string(),
	current_time: z.number(),
	width: z.number().int().nonnegative(),
	height: z.number().int().nonnegative(),
	image_base64: z.string(),
});

type Result = z.infer<typeof Out>;

/// Single canvas reused across captures — `drawImage` to an existing
/// canvas is cheap; recreating it per call would just churn GC.
const CANVAS = document.createElement('canvas');

export async function executeGetCurrentFrame(): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const player = requirePlayer();
	CANVAS.width = player.videoWidth;
	CANVAS.height = player.videoHeight;
	const ctx = CANVAS.getContext('2d');
	if (!ctx) throw new Error('2D canvas context unavailable');
	ctx.drawImage(player, 0, 0, CANVAS.width, CANVAS.height);
	return {
		video_id: videoId,
		current_time: player.currentTime,
		width: CANVAS.width,
		height: CANVAS.height,
		image_base64: CANVAS.toDataURL('image/png').split(',')[1],
	};
}

export const getCurrentFrame: Tool<typeof Args, Result> = {
	descriptor: {
		name: 'youtube_get_current_frame',
		description:
			'Capture the visible YouTube video frame (watch or shorts) as a PNG (base64-encoded), along with frame dimensions and the playback timestamp at capture. Useful for grounding the model in what the user is actually seeing on the video right now.',
		parameters: zodToJsonSchema(Args) as Record<string, unknown>,
		output_schema: zodToJsonSchema(Out) as Record<string, unknown>,
		timeout_ms: 3_000,
		source: { kind: 'bridge', app_kind: 'browser' },
		required_contexts: [],
		requires_user_approval: false,
	},
	argsSchema: Args,
	async run(_args) {
		return await executeGetCurrentFrame();
	},
};
