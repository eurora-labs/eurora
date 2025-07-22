import { signOut } from '$lib/auth.js';
import type { Actions } from './$types';

export const actions = { default: signOut } satisfies Actions;
