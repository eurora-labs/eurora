import { ProtoArticleState } from '@eurora/proto/tauri_ipc';
import { ProtoNativeArticleAsset } from '@eurora/proto/native_messaging';
import { Readability } from '@mozilla/readability';

(() => {
	console.log('Article Watcher content script loaded');

	window.addEventListener('load', () => {
		const clone = document.cloneNode(true) as Document;
		const article = new Readability(clone).parse();

		console.log('Parsed article:', article);
	});

	// Listen for messages from the extension
	chrome.runtime.onMessage.addListener((obj, sender, response) => {
		const { type } = obj;

		switch (type) {
			case 'NEW':
				// const article = new Readability(document).parse();
				// console.log('New article:', article);
				break;
			case 'GENERATE_ASSETS':
				console.log('Generating article report for URL:', window.location.href);

				const clone = document.cloneNode(true) as Document;
				const article = new Readability(clone).parse();

				console.log('Parsed article:', article);

				// Prepare report data
				const reportData: ProtoNativeArticleAsset = {
					type: 'ARTICLE_ASSET',
					content: article.content,
					textContent: article.textContent,

					title: article.title,
					siteName: article.siteName,
					language: article.lang,
					excerpt: article.excerpt,

					length: article.length
				};

				// Add selected_text if available
				const selectedText = window.getSelection()?.toString();
				if (selectedText) {
					reportData.selectedText = selectedText;
				}

				// Send response back to background script
				response(reportData);
				return true; // Important: indicates we'll send response asynchronously
			default:
				response();
		}
	});

	/**
	 * Extracts the main content from an article page
	 * This is a simple implementation that tries to find the main article content
	 * using common patterns for article pages. For production use, this would need
	 * to be more sophisticated and handle different site layouts.
	 */
	function extractArticleContent(): string {
		// Try to find content using common article containers
		const selectors = [
			'article',
			'[role="main"]',
			'.article-content',
			'.post-content',
			'.entry-content',
			'#content',
			'.content',
			'main'
		];

		for (const selector of selectors) {
			const element = document.querySelector(selector);
			if (element) {
				// Remove any script tags, ads, etc.
				const clonedElement = element.cloneNode(true) as HTMLElement;
				const scriptsAndAds = clonedElement.querySelectorAll(
					'script, style, iframe, .ad, .advertisement, .sidebar, nav, header, footer'
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
			'header, footer, nav, aside, script, style, iframe, .ad, .advertisement, .sidebar'
		);
		nonContentElements.forEach((el) => el.remove());

		return body.textContent?.trim() || '';
	}
})();
