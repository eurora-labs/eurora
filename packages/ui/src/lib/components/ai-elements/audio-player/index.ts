import Root from './AudioPlayer.svelte';
import Element from './AudioPlayerElement.svelte';
import ControlBar from './AudioPlayerControlBar.svelte';
import PlayButton from './AudioPlayerPlayButton.svelte';
import SeekBackwardButton from './AudioPlayerSeekBackwardButton.svelte';
import SeekForwardButton from './AudioPlayerSeekForwardButton.svelte';
import TimeDisplay from './AudioPlayerTimeDisplay.svelte';
import TimeRange from './AudioPlayerTimeRange.svelte';
import DurationDisplay from './AudioPlayerDurationDisplay.svelte';
import MuteButton from './AudioPlayerMuteButton.svelte';
import VolumeRange from './AudioPlayerVolumeRange.svelte';

export {
	Root,
	Element,
	ControlBar,
	PlayButton,
	SeekBackwardButton,
	SeekForwardButton,
	TimeDisplay,
	TimeRange,
	DurationDisplay,
	MuteButton,
	VolumeRange,
	//
	Root as AudioPlayer,
	Element as AudioPlayerElement,
	ControlBar as AudioPlayerControlBar,
	PlayButton as AudioPlayerPlayButton,
	SeekBackwardButton as AudioPlayerSeekBackwardButton,
	SeekForwardButton as AudioPlayerSeekForwardButton,
	TimeDisplay as AudioPlayerTimeDisplay,
	TimeRange as AudioPlayerTimeRange,
	DurationDisplay as AudioPlayerDurationDisplay,
	MuteButton as AudioPlayerMuteButton,
	VolumeRange as AudioPlayerVolumeRange,
};
