import { ProtoArticleState } from '@eurora/proto/tauri_ipc';
import { Readability } from '@mozilla/readability';
interface ArticleState extends Partial<ProtoArticleState> {
	type: 'ARTICLE_STATE';
}

(() => {
	console.log('Article Watcher content script loaded');

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

				// Prepare report data
				const reportData: ArticleState = {
					type: 'ARTICLE_STATE',
					url: window.location.href,
					title: document.title,
					// content: article?.textContent ?? document.body.textContent,
					content: document.body.textContent,
					selectedText: window.getSelection().toString()
					// ...article
				};

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
