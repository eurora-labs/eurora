import type { ParseResult } from './types';
import type { NativeImage, NativeTwitterTweet } from '../../../../shared/content/bindings';

async function fetchImage(url: string): Promise<NativeImage | null> {
	try {
		const resp = await fetch(url);
		const blob = await resp.blob();
		const dataUrl = await new Promise<string>((resolve, reject) => {
			const reader = new FileReader();
			reader.onloadend = () => resolve(reader.result as string);
			reader.onerror = reject;
			reader.readAsDataURL(blob);
		});
		const match = dataUrl.match(/^data:(image\/[^;]+);base64,(.+)$/);
		if (!match) return null;
		return { base64: match[2], mime_type: match[1] };
	} catch {
		return null;
	}
}

export abstract class BasePageParser {
	abstract parse(doc: Document): Promise<ParseResult>;

	protected async extractTweets(doc: Document): Promise<NativeTwitterTweet[]> {
		const tweets: NativeTwitterTweet[] = [];
		const tweetArticles = doc.querySelectorAll('article[data-testid="tweet"]');

		for (const article of Array.from(tweetArticles)) {
			const tweetTextEl = article.querySelector('[data-testid="tweetText"]');
			if (!tweetTextEl?.textContent) continue;

			const images: NativeImage[] = [];
			const imgElements = Array.from(
				article.querySelectorAll('[data-testid="tweetPhoto"] img'),
			);
			for (const img of imgElements) {
				const src = (img as HTMLImageElement).src;
				if (!src) continue;
				const image = await fetchImage(src);
				if (image) images.push(image);
			}

			const timestamp = article.querySelector('time')?.getAttribute('datetime') ?? null;
			const author =
				article.querySelector('a[tabindex="-1"][role="link"] span')?.textContent?.trim() ??
				null;

			tweets.push({
				text: tweetTextEl.textContent.trim(),
				timestamp,
				author,
				images,
			});
		}

		return tweets;
	}
}
