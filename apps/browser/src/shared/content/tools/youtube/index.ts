import type { z } from 'zod';
import { getCurrentFrame } from './get_current_frame';
import { getCurrentTimestamp } from './get_current_timestamp';
import { getTranscript } from './get_transcript';
import type { Tool } from '../types';

export { getCurrentFrame, getCurrentTimestamp, getTranscript };

/// YouTube watch-page tools surfaced in addition to the generic web
/// tools when the user is on `/watch`. The youtube.com watcher composes
/// `[...webTools, ...youtubeWatchTools]` and returns the combined list
/// from `listTools`.
export const youtubeWatchTools: readonly Tool<z.ZodTypeAny, unknown>[] = [
	getCurrentTimestamp,
	getTranscript,
	getCurrentFrame,
] as const;
