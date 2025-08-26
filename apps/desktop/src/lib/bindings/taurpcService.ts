import { createTauRPCProxy } from '$lib/bindings/bindings.js';
import { InjectionToken } from '@eurora/shared/context';

type TaurpcService = ReturnType<typeof createTauRPCProxy>;

export const TAURPC_SERVICE = new InjectionToken<TaurpcService>('TaurpcService');
