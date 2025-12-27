import { YouTubeTranscriptApi } from './transcript-api.js';
import browser from 'webextension-polyfill';

class YouTubeTranscriptExtractor {
	private api: YouTubeTranscriptApi;
	private currentVideoId: string | null = null;

	constructor() {
		this.api = new YouTubeTranscriptApi();
		this.init();
	}

	private init(): void {
		// Listen for messages from popup
		browser.runtime.onMessage.addListener((request, sender, sendResponse) => {
			this.handleMessage(request, sender, sendResponse);
			return true; // Keep message channel open for async response
		});

		// Monitor for video changes
		this.monitorVideoChanges();
	}

	private async handleMessage(
		request: any,
		sender: any,
		sendResponse: (response: any) => void,
	): Promise<void> {
		try {
			switch (request.action) {
				case 'getCurrentVideoId': {
					const videoId = this.getCurrentVideoId();
					sendResponse({ success: true, videoId });
					break;
				}

				case 'getTranscriptList': {
					const transcriptList = await this.api.list(request.videoId);
					const transcripts = transcriptList.getAllTranscripts().map((t) => ({
						language: t.language,
						languageCode: t.languageCode,
						isGenerated: t.isGenerated,
						isTranslatable: t.isTranslatable,
					}));
					sendResponse({ success: true, transcripts });
					break;
				}

				case 'fetchTranscript': {
					const transcript = await this.api.fetch(
						request.videoId,
						request.languages || ['en'],
						request.preserveFormatting || false,
					);
					sendResponse({ success: true, transcript });
					break;
				}

				default:
					sendResponse({ success: false, error: 'Unknown action' });
			}
		} catch (error) {
			console.error('YouTube Transcript Extractor error:', error);
			sendResponse({
				success: false,
				error: error instanceof Error ? error.message : 'Unknown error',
			});
		}
	}

	private getCurrentVideoId(): string | null {
		// Method 1: Extract from URL
		const urlParams = new URLSearchParams(window.location.search);
		const videoId = urlParams.get('v');

		if (videoId) {
			return videoId;
		}

		// Method 2: Extract from page data
		try {
			const ytInitialData = this.extractYtInitialData();
			if (ytInitialData?.currentVideoEndpoint?.watchEndpoint?.videoId) {
				return ytInitialData.currentVideoEndpoint.watchEndpoint.videoId;
			}
		} catch (e) {
			console.warn('Could not extract video ID from ytInitialData:', e);
		}

		// Method 3: Extract from player data
		try {
			const scripts = document.querySelectorAll('script');
			for (const script of Array.from(scripts)) {
				const content = script.textContent || '';
				const match = content.match(/"videoId":"([a-zA-Z0-9_-]{11})"/);
				if (match) {
					return match[1];
				}
			}
		} catch (e) {
			console.warn('Could not extract video ID from scripts:', e);
		}

		return null;
	}

	private extractYtInitialData(): any {
		try {
			const scripts = document.querySelectorAll('script');
			for (const script of Array.from(scripts)) {
				const content = script.textContent || '';
				const match = content.match(/var ytInitialData = ({.+?});/);
				if (match) {
					return JSON.parse(match[1]);
				}
			}
		} catch (e) {
			console.warn('Could not parse ytInitialData:', e);
		}
		return null;
	}

	private monitorVideoChanges(): void {
		let lastVideoId = this.getCurrentVideoId();
		this.currentVideoId = lastVideoId;

		// Monitor URL changes (for SPA navigation)
		const observer = new MutationObserver(() => {
			const currentVideoId = this.getCurrentVideoId();
			if (currentVideoId && currentVideoId !== lastVideoId) {
				lastVideoId = currentVideoId;
				this.currentVideoId = currentVideoId;
				this.notifyVideoChange(currentVideoId);
			}
		});

		observer.observe(document.body, {
			childList: true,
			subtree: true,
		});

		// Also listen for popstate events
		window.addEventListener('popstate', () => {
			setTimeout(() => {
				const currentVideoId = this.getCurrentVideoId();
				if (currentVideoId && currentVideoId !== lastVideoId) {
					lastVideoId = currentVideoId;
					this.currentVideoId = currentVideoId;
					this.notifyVideoChange(currentVideoId);
				}
			}, 100);
		});
	}

	private notifyVideoChange(videoId: string): void {
		// Send message to background script about video change
		browser.runtime
			.sendMessage({
				action: 'videoChanged',
				videoId: videoId,
			})
			.catch(() => {
				// Ignore errors if background script is not listening
			});
	}
}

// Initialize the extractor when the page loads
if (document.readyState === 'loading') {
	document.addEventListener('DOMContentLoaded', () => {
		new YouTubeTranscriptExtractor();
	});
} else {
	new YouTubeTranscriptExtractor();
}
