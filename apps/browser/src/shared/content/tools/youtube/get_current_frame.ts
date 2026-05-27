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
/// Lazily created on first capture so the module can be loaded outside
/// a browser (e.g. by the e2e test process that imports tool
/// descriptors for type derivation).
let canvas: HTMLCanvasElement | null = null;

function getCanvas(): HTMLCanvasElement {
	if (canvas === null) canvas = document.createElement('canvas');
	return canvas;
}

export async function executeGetCurrentFrame(): Promise<Result> {
	const videoId = requireCurrentVideoId();
	const player = requirePlayer();
	const target = getCanvas();
	target.width = player.videoWidth;
	target.height = player.videoHeight;
	const ctx = target.getContext('2d');
	if (!ctx) throw new Error('2D canvas context unavailable');
	ctx.drawImage(player, 0, 0, target.width, target.height);
	return {
		video_id: videoId,
		current_time: player.currentTime,
		width: target.width,
		height: target.height,
		image_base64: target.toDataURL('image/png').split(',')[1],
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
