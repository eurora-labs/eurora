// Overlay windows opt out of the root `+layout.ts` consent gate (the
// gate event is delivered only after `frontendReady`, which only the
// main window issues — overlays would hang forever waiting on it).
// They also opt out of SSR / prerender, matching the root.
export const prerender = false;
export const ssr = false;
