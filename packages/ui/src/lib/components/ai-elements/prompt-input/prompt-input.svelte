<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLFormAttributes } from 'svelte/elements';
	import { onMount, onDestroy } from 'svelte';
	import { cn } from '$lib/utils.js';
	import { InputGroup } from '$lib/components/input-group/index.js';
	import { nanoid } from 'nanoid';
	import {
		AttachmentsState,
		ReferencedSourcesState,
		setLocalAttachments,
		setReferencedSources,
		useOptionalPromptInputController,
		type FileUIPart,
		type PromptInputMessage,
	} from './prompt-input-context.svelte.js';

	let {
		class: className,
		accept = undefined,
		multiple = false,
		globalDrop = false,
		maxFiles = undefined,
		maxFileSize = undefined,
		onError = undefined,
		onSubmit,
		children,
		...restProps
	}: Omit<HTMLFormAttributes, 'onsubmit'> & {
		accept?: string;
		multiple?: boolean;
		globalDrop?: boolean;
		maxFiles?: number;
		maxFileSize?: number;
		onError?: (err: {
			code: 'max_files' | 'max_file_size' | 'accept';
			message: string;
		}) => void;
		onSubmit: (message: PromptInputMessage, event: SubmitEvent) => void | Promise<void>;
		children?: Snippet;
	} = $props();

	const controller = useOptionalPromptInputController();
	const usingProvider = !!controller;

	let inputEl: HTMLInputElement | null = $state(null);
	let formEl: HTMLFormElement | null = $state(null);

	const localAttachments = new AttachmentsState();
	const referencedSources = new ReferencedSourcesState();

	const files = $derived(usingProvider ? controller!.attachments.files : localAttachments.files);

	setReferencedSources(referencedSources);

	function matchesAccept(f: File): boolean {
		if (!accept || accept.trim() === '') return true;
		const patterns = accept
			.split(',')
			.map((s) => s.trim())
			.filter(Boolean);
		return patterns.some((pattern) => {
			if (pattern.endsWith('/*')) {
				const prefix = pattern.slice(0, -1);
				return f.type.startsWith(prefix);
			}
			return f.type === pattern;
		});
	}

	function validateAndAdd(fileList: File[] | FileList) {
		const incoming = [...fileList];
		const accepted = incoming.filter((f) => matchesAccept(f));
		if (incoming.length && accepted.length === 0) {
			onError?.({ code: 'accept', message: 'No files match the accepted types.' });
			return;
		}
		const withinSize = (f: File) => (maxFileSize ? f.size <= maxFileSize : true);
		const sized = accepted.filter(withinSize);
		if (accepted.length > 0 && sized.length === 0) {
			onError?.({ code: 'max_file_size', message: 'All files exceed the maximum size.' });
			return;
		}

		const currentCount = files.length;
		const capacity =
			typeof maxFiles === 'number' ? Math.max(0, maxFiles - currentCount) : undefined;
		const capped = typeof capacity === 'number' ? sized.slice(0, capacity) : sized;
		if (typeof capacity === 'number' && sized.length > capacity) {
			onError?.({ code: 'max_files', message: 'Too many files. Some were not added.' });
		}

		if (capped.length === 0) return;

		if (usingProvider) {
			controller!.attachments.add(capped);
		} else {
			const newFiles: FileUIPart[] = capped.map((file) => ({
				filename: file.name,
				id: nanoid(),
				mediaType: file.type,
				type: 'file' as const,
				url: URL.createObjectURL(file),
			}));
			localAttachments.files = [...localAttachments.files, ...newFiles];
		}
	}

	function removeFile(id: string) {
		if (usingProvider) {
			controller!.attachments.remove(id);
		} else {
			localAttachments.remove(id);
		}
	}

	function clearAttachments() {
		if (usingProvider) {
			controller!.attachments.clear();
		} else {
			localAttachments.clear();
		}
	}

	function clearAll() {
		clearAttachments();
		referencedSources.clear();
	}

	function openFileDialog() {
		inputEl?.click();
	}

	// Expose a context-friendly attachments object that delegates to the
	// correct backing store while applying validation from this component.
	const contextAttachments = new AttachmentsState();
	// Override the backing state via $effect so reads are always current
	$effect(() => {
		contextAttachments.files = files;
	});
	// Patch methods to route through validation / correct store
	Object.defineProperties(contextAttachments, {
		add: { value: (incoming: File[] | FileList) => validateAndAdd(incoming), writable: false },
		remove: { value: (id: string) => removeFile(id), writable: false },
		clear: { value: () => clearAttachments(), writable: false },
		openFileDialog: { value: () => openFileDialog(), writable: false },
	});

	setLocalAttachments(contextAttachments);

	if (usingProvider) {
		$effect(() => {
			if (inputEl) {
				controller!.registerFileInput(inputEl, () => inputEl?.click());
			}
		});
	}

	function handleFileChange(event: Event) {
		const target = event.currentTarget as HTMLInputElement;
		if (target.files) {
			validateAndAdd(target.files);
		}
		target.value = '';
	}

	const convertBlobUrlToDataUrl = async (url: string): Promise<string | null> => {
		try {
			const response = await fetch(url);
			const blob = await response.blob();
			return new Promise((resolve) => {
				const reader = new FileReader();
				reader.onloadend = () => resolve(reader.result as string);
				reader.onerror = () => resolve(null);
				reader.readAsDataURL(blob);
			});
		} catch {
			return null;
		}
	};

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();

		const form = event.currentTarget as HTMLFormElement;
		const text = usingProvider
			? controller!.textInput.value
			: (() => {
					const formData = new FormData(form);
					return (formData.get('message') as string) || '';
				})();

		if (!usingProvider) {
			form.reset();
		}

		try {
			const convertedFiles: Omit<FileUIPart, 'id'>[] = await Promise.all(
				files.map(async ({ id: _id, ...item }) => {
					if (item.url?.startsWith('blob:')) {
						const dataUrl = await convertBlobUrlToDataUrl(item.url);
						return { ...item, url: dataUrl ?? item.url };
					}
					return item;
				}),
			);

			const result = onSubmit({ files: convertedFiles, text }, event);

			if (result instanceof Promise) {
				try {
					await result;
					clearAll();
					if (usingProvider) {
						controller!.textInput.clear();
					}
				} catch {
					// Don't clear on error
				}
			} else {
				clearAll();
				if (usingProvider) {
					controller!.textInput.clear();
				}
			}
		} catch {
			// Don't clear on error
		}
	}

	onMount(() => {
		const form = formEl;
		if (!form) return;

		const onDragOver = (e: DragEvent) => {
			if (e.dataTransfer?.types?.includes('Files')) {
				e.preventDefault();
			}
		};
		const onDrop = (e: DragEvent) => {
			if (e.dataTransfer?.types?.includes('Files')) {
				e.preventDefault();
			}
			if (e.dataTransfer?.files && e.dataTransfer.files.length > 0) {
				validateAndAdd(e.dataTransfer.files);
			}
		};

		const target = globalDrop ? document : form;
		target.addEventListener('dragover', onDragOver as EventListener);
		target.addEventListener('drop', onDrop as EventListener);

		return () => {
			target.removeEventListener('dragover', onDragOver as EventListener);
			target.removeEventListener('drop', onDrop as EventListener);
		};
	});

	onDestroy(() => {
		if (!usingProvider) {
			localAttachments.destroy();
		}
	});
</script>

<input
	bind:this={inputEl}
	{accept}
	aria-label="Upload files"
	class="hidden"
	{multiple}
	onchange={handleFileChange}
	title="Upload files"
	type="file"
/>
<form
	bind:this={formEl}
	data-slot="prompt-input"
	class={cn('w-full', className)}
	onsubmit={handleSubmit}
	{...restProps}
>
	<InputGroup class="overflow-hidden">
		{@render children?.()}
	</InputGroup>
</form>
