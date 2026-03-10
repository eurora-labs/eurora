<script lang="ts">
	import {
		getDownloadUrlForOS,
		type OSType,
		type ArchType,
	} from '$lib/services/download-service';
	import { getArch, getOS, getOSDisplayName } from '$lib/utils/getOS';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import ArrowRight from '@lucide/svelte/icons/arrow-right';

	interface Props {
		class?: string;
	}

	let { class: className = '' }: Props = $props();

	let os = $state<OSType>('unknown');
	let arch = $state<ArchType>('unknown');

	$effect(() => {
		os = getOS();
		arch = getArch();
	});

	function handleDownload() {
		if (os === 'unknown') {
			window.location.href = '/download';
			return;
		}

		window.location.href = getDownloadUrlForOS(os, arch);
	}
</script>

<button class="download-btn {className}" onclick={handleDownload}>
	<span class="download-btn-bg"></span>
	<span class="download-btn-orb download-btn-orb-1"></span>
	<span class="download-btn-orb download-btn-orb-2"></span>
	<span class="download-btn-orb download-btn-orb-3"></span>
	<span class="download-btn-content">
		<DownloadIcon class="size-8" />
		<span class="download-btn-text">
			<span class="download-btn-label">Download for {getOSDisplayName(os)}</span>
			<span class="download-btn-hint"
				>Free & open source <ArrowRight class="inline size-4" /></span
			>
		</span>
	</span>
</button>

<style>
	.download-btn {
		display: flex;
		position: relative;
		align-items: center;
		justify-content: flex-start;
		padding-left: 1.5rem;
		overflow: hidden;
		border: 1px solid rgba(255, 255, 255, 0.15);
		border-radius: 1rem;
		box-shadow:
			0 4px 24px rgba(59, 130, 246, 0.4),
			0 0 0 0 rgba(59, 130, 246, 0.3);
		color: #fff;
		animation: pulse-ring 5s ease-in-out infinite;
		cursor: pointer;
		transition:
			transform 0.3s cubic-bezier(0.4, 0, 0.2, 1),
			box-shadow 0s;
	}

	.download-btn:hover {
		transform: scale(1.02);
		box-shadow:
			0 8px 40px rgba(59, 130, 246, 0.5),
			0 0 60px rgba(139, 92, 246, 0.2);
		animation: none;
	}

	.download-btn:active {
		transform: scale(0.99);
	}

	@keyframes pulse-ring {
		0%,
		100% {
			box-shadow:
				0 4px 24px rgba(59, 130, 246, 0.4),
				0 0 0 0 rgba(59, 130, 246, 0.3);
		}
		50% {
			box-shadow:
				0 4px 24px rgba(59, 130, 246, 0.4),
				0 0 0 10px rgba(59, 130, 246, 0);
		}
	}

	.download-btn-bg {
		position: absolute;
		inset: 0;
		background: linear-gradient(135deg, #1e3a5f 0%, #2563eb 40%, #7c3aed 70%, #1e3a5f 100%);
		background-size: 300% 300%;
		animation: gradient-shift 14s ease-in-out infinite;
	}

	@keyframes gradient-shift {
		0%,
		100% {
			background-position: 0% 50%;
		}
		33% {
			background-position: 100% 0%;
		}
		66% {
			background-position: 50% 100%;
		}
	}

	.download-btn-orb {
		position: absolute;
		border-radius: 50%;
		mix-blend-mode: screen;
		filter: blur(30px);
		opacity: 0.5;
	}

	.download-btn-orb-1 {
		width: 120px;
		height: 120px;
		background: #60a5fa;
		animation: orb-float-1 12s ease-in-out infinite;
	}

	.download-btn-orb-2 {
		width: 100px;
		height: 100px;
		background: #a78bfa;
		animation: orb-float-2 16s ease-in-out infinite;
	}

	.download-btn-orb-3 {
		width: 80px;
		height: 80px;
		background: #34d399;
		animation: orb-float-3 14s ease-in-out infinite;
	}

	@keyframes orb-float-1 {
		0%,
		100% {
			transform: translate(-30%, -40%);
		}
		33% {
			transform: translate(60%, -20%);
		}
		66% {
			transform: translate(20%, 40%);
		}
	}

	@keyframes orb-float-2 {
		0%,
		100% {
			transform: translate(80%, 50%);
		}
		33% {
			transform: translate(-20%, -30%);
		}
		66% {
			transform: translate(50%, -50%);
		}
	}

	@keyframes orb-float-3 {
		0%,
		100% {
			transform: translate(40%, 60%);
		}
		33% {
			transform: translate(-40%, 20%);
		}
		66% {
			transform: translate(70%, -40%);
		}
	}

	.download-btn-content {
		display: flex;
		z-index: 1;
		position: relative;
		align-items: center;
		gap: 0.75rem;
		text-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
	}

	.download-btn-text {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.125rem;
	}

	.download-btn-label {
		font-weight: 700;
		font-size: 1.125rem;
		letter-spacing: -0.01em;
	}

	.download-btn-hint {
		font-weight: 400;
		font-size: 0.8rem;
		opacity: 0.85;
	}
</style>
