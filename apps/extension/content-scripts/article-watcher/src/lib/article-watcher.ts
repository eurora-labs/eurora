import { Watcher } from '@eurora/chrome-ext-shared/extensions/watchers/watcher';
import { ArticleChromeMessage, type ArticleMessageType, type WatcherParams } from './types.js';
import { NativeArticleAsset, NativeArticleSnapshot } from '@eurora/chrome-ext-shared/bindings';
import { Readability } from '@mozilla/readability';

class ArticleWatcher extends Watcher<WatcherParams> {
	constructor(params: WatcherParams) {
		super(params);
	}

	public listen(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const { type } = obj;

		switch (type) {
			case 'NEW':
				this.handleNew(obj, sender, response);
				break;
			case 'GENERATE_ASSETS':
				this.handleGenerateAssets(obj, sender, response);
				break;
			case 'GENERATE_SNAPSHOT':
				this.handleGenerateSnapshot(obj, sender, response);
				break;
			default:
				response();
		}
	}

	public handleNew(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		console.log('Article Watcher: New article detected');
		// Parse article on page load for caching
		const clone = document.cloneNode(true) as Document;
		const article = new Readability(clone).parse();
		console.log('Parsed article:', article);
		response();
	}

	public handleGenerateAssets(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		console.log('Generating article report for URL:', window.location.href);

		try {
			const clone = document.cloneNode(true) as Document;
			const article = new Readability(clone).parse();

			console.log('Parsed article:', article);

			const reportData: NativeArticleAsset = {
				content: article?.content || '',
				text_content: article?.textContent || '',
				title: article?.title || document.title,
				site_name: article?.siteName || '',
				language: article?.lang || '',
				excerpt: article?.excerpt || '',
				length: article?.length || 0,
				selected_text: window.getSelection()?.toString() || '',
			};

			// Send response back to background script
			response({ kind: 'NativeArticleAsset', data: reportData });
		} catch (error) {
			const errorMessage = error instanceof Error ? error.message : String(error);
			const contextualError = `Failed to generate article assets for ${window.location.href}: ${errorMessage}`;
			console.error('Error generating article report:', {
				url: window.location.href,
				error: errorMessage,
				stack: error instanceof Error ? error.stack : undefined,
			});
			response({
				success: false,
				error: contextualError,
				context: {
					url: window.location.href,
					timestamp: new Date().toISOString(),
				},
			});
		}

		return true; // Important: indicates we'll send response asynchronously
	}

	public handleGenerateSnapshot(
		obj: ArticleChromeMessage,
		sender: chrome.runtime.MessageSender,
		response: (response?: any) => void,
	) {
		const selectedText = window.getSelection()?.toString() || '';
		const snapshot: NativeArticleSnapshot = {
			highlighted_text: selectedText,
		};

		response({ kind: 'NativeArticleSnapshot', data: snapshot });
		return true;
	}

	/**
	 * Extracts the main content from an article page
	 * This is a simple implementation that tries to find the main article content
	 * using common patterns for article pages. For production use, this would need
	 * to be more sophisticated and handle different site layouts.
	 */
	private extractArticleContent(): string {
		// Try to find content using common article containers
		const selectors = [
			'article',
			'[role="main"]',
			'.article-content',
			'.post-content',
			'.entry-content',
			'#content',
			'.content',
			'main',
		];

		for (const selector of selectors) {
			const element = document.querySelector(selector);
			if (element) {
				// Remove any script tags, ads, etc.
				const clonedElement = element.cloneNode(true) as HTMLElement;
				const scriptsAndAds = clonedElement.querySelectorAll(
					'script, style, iframe, .ad, .advertisement, .sidebar, nav, header, footer',
				);

				scriptsAndAds.forEach((el) => el.remove());

				// Get the text content
				return clonedElement.textContent?.trim() || '';
			}
		}

		// Fallback: if no article container is found, try to extract from the body
		// but exclude common non-content elements
		const body = document.body.cloneNode(true) as HTMLElement;
		const nonContentElements = body.querySelectorAll(
			'header, footer, nav, aside, script, style, iframe, .ad, .advertisement, .sidebar',
		);
		nonContentElements.forEach((el) => el.remove());

		return body.textContent?.trim() || '';
	}
}

(() => {
	console.log('Article Watcher content script loaded');

	const watcher = new ArticleWatcher({});

	// Parse article on page load
	window.addEventListener('load', () => {
		const clone = document.cloneNode(true) as Document;
		const article = new Readability(clone).parse();
		console.log('Parsed article on load:', article);
	});

	chrome.runtime.onMessage.addListener(watcher.listen.bind(watcher));
})();
