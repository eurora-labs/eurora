/// Shared YouTube-page helpers used by the watch-page tool set. Kept
/// thin — only resolution of the per-page `<video>` element and the
/// canonical video id. Tools themselves stay self-contained otherwise.

export function getCurrentVideoId(): string | undefined {
	if (window.location.search?.includes('v=')) {
		return window.location.search.split('v=')[1].split('&')[0];
	}
	return undefined;
}

export function requireCurrentVideoId(): string {
	const videoId = getCurrentVideoId();
	if (videoId === undefined) {
		throw new Error('YouTube watch page has no video id');
	}
	return videoId;
}

/// Resolve the `<video>` element. Throws when the page has no player or
/// the player hasn't loaded enough data to read `currentTime` /
/// `duration` (`readyState === 0`).
export function requirePlayer(): HTMLVideoElement {
	const player = document.querySelector<HTMLVideoElement>('video.html5-main-video');
	if (!player) throw new Error('no YouTube player element on the page');
	if (player.readyState === 0) throw new Error('YouTube player not ready');
	return player;
}
