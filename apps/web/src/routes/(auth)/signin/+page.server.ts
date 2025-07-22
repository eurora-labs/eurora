import { signIn } from '../../../auth.js';
import type { Actions } from './$types';

export const actions = { default: signIn } satisfies Actions;
