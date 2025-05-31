<script lang="ts">
	import {
		Calendar,
		Smile,
		Calculator,
		User,
		CreditCard,
		Settings,
		Captions,
		Film,
		YoutubeIcon,
		HardDrive,
		FileTextIcon
	} from '@lucide/svelte';

	import { SiGoogledrive } from '@icons-pack/svelte-simple-icons';

	// import { LauncherNative as Launcher } from '@eurora/launcher';
	import { Launcher } from '@eurora/launcher';
	import { transcriptExtension } from '@eurora/ext-transcript';
	import { Editor as ProsemirrorEditor, TextSelection } from '@eurora/prosemirror-core';
	let editorRef: ProsemirrorEditor | undefined = $state(undefined);

	let exampleInput = $state({
		text: '',
		extensions: [transcriptExtension()]
	});

	function addExerciseSheet() {
		editorRef?.cmd((state, dispatch) => {
			const tr = state.tr;
			const { schema } = state;
			const nodes = schema.nodes;
			const { $from: from } = state.selection;
			tr.insert(
				from.pos,
				nodes.transcript.createChecked(
					{ id: 'transcript-1', text: 'Exercise Sheet 2' },
					schema.text(' ')
				)
			);

			dispatch?.(tr);
		});
	}
</script>

<div>
	<div class="launcher absolute left-1/2 top-1/4 w-[1100px] -translate-x-1/2">
		<Launcher.Root class="rounded-lg border shadow-md">
			<Launcher.Input placeholder="Search" bind:query={exampleInput} bind:editorRef />
			<!-- <span class="absolute left-[175px] top-4 ml-2 mt-2 flex items-center gap-2">
				<div class="transcript-badge">transcript</div>
				<div class="transcript-badge">video</div>
			</span> -->
			<Launcher.List>
				<!-- <Launcher.Empty>No results found.</Launcher.Empty> -->
				<Launcher.Group heading="Local Files">
					<Launcher.Item onclick={addExerciseSheet}>
						<HardDrive />
						<span>Exercise Sheet 2</span>
					</Launcher.Item>
					<Launcher.Item>
						<FileTextIcon />
						<span>Notes</span>
					</Launcher.Item>
				</Launcher.Group>
				<Launcher.Separator />
				<Launcher.Group heading="Google Drive">
					<Launcher.Item>
						<SiGoogledrive />
						<span>Presentation 1</span>
					</Launcher.Item>
					<Launcher.Item>
						<SiGoogledrive />
						<span>Report card</span>
					</Launcher.Item>
					<Launcher.Item>
						<SiGoogledrive />
						<span>Exercise sheet 3</span>
					</Launcher.Item>
				</Launcher.Group>
			</Launcher.List>
		</Launcher.Root>
		<!-- <Search /> -->
		<!-- <Input type="text" placeholder="Eurora Search" class="h-full w-full border-none text-[32px]" /> -->
		<!-- <Launcher /> -->
	</div>
	<img class=" w-full" src="/sample_background.jpg" alt="Sample Background" />
</div>

<style lang="postcss">
	.launcher {
		background: rgba(255, 255, 255, 0.2);
		box-shadow: 0 4px 30px rgba(0, 0, 0, 0.1);
		backdrop-filter: blur(36px);
		-webkit-backdrop-filter: blur(36px);
	}

	.transcript-badge {
		@apply flex w-fit items-center gap-2 text-[40px] leading-[40px] text-white;
		@apply p-2;
		background: rgba(128, 128, 128, 0.4);
		color: rgba(0, 0, 0, 0.8);
		border-radius: 16px;
	}
</style>
