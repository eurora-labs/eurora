<script lang="ts" module>
	import type { HTMLButtonAttributes } from 'svelte/elements';

	interface SpeechRecognitionInstance extends EventTarget {
		continuous: boolean;
		interimResults: boolean;
		lang: string;
		start(): void;
		stop(): void;
	}

	interface SpeechRecognitionResultItem {
		transcript: string;
		confidence: number;
	}

	interface SpeechRecognitionResult {
		readonly length: number;
		isFinal: boolean;
		[index: number]: SpeechRecognitionResultItem;
	}

	interface SpeechRecognitionResultList {
		readonly length: number;
		[index: number]: SpeechRecognitionResult;
	}

	interface SpeechRecognitionEvent extends Event {
		results: SpeechRecognitionResultList;
		resultIndex: number;
	}

	type SpeechInputMode = 'speech-recognition' | 'media-recorder' | 'none';

	function detectSpeechInputMode(): SpeechInputMode {
		if (typeof window === 'undefined') return 'none';
		if ('SpeechRecognition' in window || 'webkitSpeechRecognition' in window) {
			return 'speech-recognition';
		}
		if ('MediaRecorder' in window && 'mediaDevices' in navigator) {
			return 'media-recorder';
		}
		return 'none';
	}

	export interface SpeechInputProps extends HTMLButtonAttributes {
		onTranscriptionChange?: (text: string) => void;
		onAudioRecorded?: (audioBlob: Blob) => Promise<string>;
		lang?: string;
	}
</script>

<script lang="ts">
	import { cn } from '$lib/utils.js';
	import { Button } from '$lib/components/button/index.js';
	import MicIcon from '@lucide/svelte/icons/mic';
	import SquareIcon from '@lucide/svelte/icons/square';
	import LoaderCircleIcon from '@lucide/svelte/icons/loader-circle';
	import { onMount, onDestroy } from 'svelte';

	let {
		class: className,
		onTranscriptionChange,
		onAudioRecorded,
		lang = 'en-US',
		...restProps
	}: SpeechInputProps = $props();

	let isListening = $state(false);
	let isProcessing = $state(false);
	let mode = $state<SpeechInputMode>('none');
	let isRecognitionReady = $state(false);

	let recognition: SpeechRecognitionInstance | null = null;
	let mediaRecorder: MediaRecorder | null = null;
	let stream: MediaStream | null = null;
	let audioChunks: Blob[] = [];

	let isDisabled = $derived(
		mode === 'none' ||
			(mode === 'speech-recognition' && !isRecognitionReady) ||
			(mode === 'media-recorder' && !onAudioRecorded) ||
			isProcessing,
	);

	onMount(() => {
		mode = detectSpeechInputMode();

		if (mode === 'speech-recognition') {
			const SpeechRecognitionCtor =
				(window as any).SpeechRecognition || (window as any).webkitSpeechRecognition;
			const sr = new SpeechRecognitionCtor();
			sr.continuous = true;
			sr.interimResults = true;
			sr.lang = lang;

			sr.addEventListener('start', () => {
				isListening = true;
			});

			sr.addEventListener('end', () => {
				isListening = false;
			});

			sr.addEventListener('result', (event: Event) => {
				const speechEvent = event as SpeechRecognitionEvent;
				let finalTranscript = '';
				for (let i = speechEvent.resultIndex; i < speechEvent.results.length; i += 1) {
					const result = speechEvent.results[i];
					if (result.isFinal) {
						finalTranscript += result[0]?.transcript ?? '';
					}
				}
				if (finalTranscript) {
					onTranscriptionChange?.(finalTranscript);
				}
			});

			sr.addEventListener('error', () => {
				isListening = false;
			});

			recognition = sr;
			isRecognitionReady = true;
		}
	});

	onDestroy(() => {
		if (recognition) {
			recognition.stop();
			recognition = null;
		}
		if (mediaRecorder?.state === 'recording') {
			mediaRecorder.stop();
		}
		if (stream) {
			for (const track of stream.getTracks()) {
				track.stop();
			}
		}
	});

	async function startMediaRecorder() {
		if (!onAudioRecorded) return;

		try {
			stream = await navigator.mediaDevices.getUserMedia({ audio: true });
			const recorder = new MediaRecorder(stream);
			audioChunks = [];

			recorder.addEventListener('dataavailable', (event: BlobEvent) => {
				if (event.data.size > 0) {
					audioChunks.push(event.data);
				}
			});

			recorder.addEventListener('stop', async () => {
				if (stream) {
					for (const track of stream.getTracks()) {
						track.stop();
					}
					stream = null;
				}

				const audioBlob = new Blob(audioChunks, { type: 'audio/webm' });
				if (audioBlob.size > 0 && onAudioRecorded) {
					isProcessing = true;
					try {
						const transcript = await onAudioRecorded(audioBlob);
						if (transcript) {
							onTranscriptionChange?.(transcript);
						}
					} catch {
						// Error handling delegated to the onAudioRecorded caller
					} finally {
						isProcessing = false;
					}
				}
			});

			recorder.addEventListener('error', () => {
				isListening = false;
				if (stream) {
					for (const track of stream.getTracks()) {
						track.stop();
					}
					stream = null;
				}
			});

			mediaRecorder = recorder;
			recorder.start();
			isListening = true;
		} catch {
			isListening = false;
		}
	}

	function stopMediaRecorder() {
		if (mediaRecorder?.state === 'recording') {
			mediaRecorder.stop();
		}
		isListening = false;
	}

	function toggleListening() {
		if (mode === 'speech-recognition' && recognition) {
			if (isListening) {
				recognition.stop();
			} else {
				recognition.start();
			}
		} else if (mode === 'media-recorder') {
			if (isListening) {
				stopMediaRecorder();
			} else {
				startMediaRecorder();
			}
		}
	}
</script>

<div data-slot="speech-input" class="relative inline-flex items-center justify-center">
	{#if isListening}
		{#each [0, 1, 2] as index}
			<div
				class="absolute inset-0 animate-ping rounded-full border-2 border-red-400/30"
				style="animation-delay: {index * 0.3}s; animation-duration: 2s;"
			></div>
		{/each}
	{/if}

	<Button
		class={cn(
			'relative z-10 rounded-full transition-all duration-300',
			isListening
				? 'bg-destructive text-white hover:bg-destructive/80 hover:text-white'
				: 'bg-primary text-primary-foreground hover:bg-primary/80 hover:text-primary-foreground',
			className,
		)}
		disabled={isDisabled}
		onclick={toggleListening}
		{...restProps}
	>
		{#if isProcessing}
			<LoaderCircleIcon class="size-4 animate-spin" />
		{:else if isListening}
			<SquareIcon class="size-4" />
		{:else}
			<MicIcon class="size-4" />
		{/if}
	</Button>
</div>
