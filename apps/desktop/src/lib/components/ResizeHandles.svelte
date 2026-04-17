<script lang="ts">
	import { getCurrentWindow } from '@tauri-apps/api/window';

	type ResizeDirection =
		| 'North'
		| 'South'
		| 'East'
		| 'West'
		| 'NorthEast'
		| 'NorthWest'
		| 'SouthEast'
		| 'SouthWest';

	const appWindow = getCurrentWindow();

	function start(direction: ResizeDirection) {
		return (event: PointerEvent) => {
			if (event.button !== 0) return;
			event.preventDefault();
			appWindow.startResizeDragging(direction);
		};
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle edge-n" onpointerdown={start('North')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle edge-s" onpointerdown={start('South')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle edge-e" onpointerdown={start('East')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle edge-w" onpointerdown={start('West')}></div>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle corner-nw" onpointerdown={start('NorthWest')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle corner-ne" onpointerdown={start('NorthEast')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle corner-sw" onpointerdown={start('SouthWest')}></div>
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="handle corner-se" onpointerdown={start('SouthEast')}></div>

<style>
	.handle {
		z-index: 50;
		position: fixed;
	}

	.edge-n {
		top: 0;
		right: 14px;
		left: 14px;
		height: 6px;
		cursor: ns-resize;
	}
	.edge-s {
		right: 14px;
		bottom: 0;
		left: 14px;
		height: 6px;
		cursor: ns-resize;
	}
	.edge-e {
		top: 14px;
		right: 0;
		bottom: 14px;
		width: 6px;
		cursor: ew-resize;
	}
	.edge-w {
		top: 14px;
		bottom: 14px;
		left: 0;
		width: 6px;
		cursor: ew-resize;
	}

	.corner-nw {
		top: 0;
		left: 0;
		width: 14px;
		height: 14px;
		cursor: nwse-resize;
	}
	.corner-ne {
		top: 0;
		right: 0;
		width: 14px;
		height: 14px;
		cursor: nesw-resize;
	}
	.corner-sw {
		bottom: 0;
		left: 0;
		width: 14px;
		height: 14px;
		cursor: nesw-resize;
	}
	.corner-se {
		right: 0;
		bottom: 0;
		width: 14px;
		height: 14px;
		cursor: nwse-resize;
	}
</style>
