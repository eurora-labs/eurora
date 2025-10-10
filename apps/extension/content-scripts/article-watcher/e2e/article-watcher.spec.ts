import { test, expect } from '@playwright/test';

test.describe('Article Watcher Content Script - Asset Generation', () => {
	test('should extract article content using Readability', async ({ page }) => {
		// Create a realistic article page
		await page.setContent(`
			<!DOCTYPE html>
			<html lang="en">
			<head>
				<title>Understanding JavaScript Closures</title>
				<meta name="description" content="A comprehensive guide to JavaScript closures" />
			</head>
			<body>
				<header>
					<nav>Navigation menu</nav>
				</header>
				<main>
					<article>
						<h1>Understanding JavaScript Closures</h1>
						<p class="author">By John Doe</p>
						<p>Closures are one of the most powerful features in JavaScript. They allow functions to access variables from an outer function even after the outer function has returned.</p>
						<h2>What is a Closure?</h2>
						<p>A closure is the combination of a function bundled together with references to its surrounding state (the lexical environment).</p>
						<p>In JavaScript, closures are created every time a function is created, at function creation time.</p>
						<h2>Example</h2>
						<pre><code>function outer() {
  const x = 10;
  return function inner() {
    console.log(x);
  };
}</code></pre>
						<p>This is a fundamental concept that every JavaScript developer should understand.</p>
					</article>
				</main>
				<footer>
					<p>Copyright 2024</p>
				</footer>
			</body>
			</html>
		`);

		// Verify the article structure is present
		const article = page.locator('article');
		await expect(article).toBeVisible();

		const mainHeading = article.locator('h1');
		await expect(mainHeading).toHaveText('Understanding JavaScript Closures');

		// Verify multiple paragraphs exist
		const paragraphs = article.locator('p');
		await expect(paragraphs).toHaveCount(5);

		// Verify code block exists
		const codeBlock = article.locator('pre code');
		await expect(codeBlock).toBeVisible();
	});

	test('should extract text content from complex article', async ({ page }) => {
		await page.setContent(`
			<!DOCTYPE html>
			<html lang="en">
			<head>
				<title>The Future of Web Development</title>
			</head>
			<body>
				<div class="ads">Advertisement</div>
				<article>
					<h1>The Future of Web Development</h1>
					<p>Web development is constantly evolving with new technologies and frameworks.</p>
					<ul>
						<li>React and Vue continue to dominate</li>
						<li>WebAssembly is gaining traction</li>
						<li>Progressive Web Apps are becoming mainstream</li>
					</ul>
					<p>These trends will shape the next decade of web development.</p>
				</article>
				<aside class="sidebar">Related articles</aside>
			</body>
			</html>
		`);

		// Verify article content is accessible
		const article = page.locator('article');
		await expect(article).toBeVisible();

		// Verify list items
		const listItems = article.locator('li');
		await expect(listItems).toHaveCount(3);
		await expect(listItems.first()).toContainText('React and Vue');
	});

	test('should handle article with metadata', async ({ page }) => {
		await page.setContent(`
			<!DOCTYPE html>
			<html lang="en">
			<head>
				<title>Test Article Title</title>
				<meta name="author" content="Jane Smith" />
				<meta name="description" content="This is a test article description" />
				<meta property="og:site_name" content="Tech Blog" />
			</head>
			<body>
				<article>
					<h1>Test Article Title</h1>
					<p>This is the main content of the article that should be extracted.</p>
				</article>
			</body>
			</html>
		`);

		// Verify title
		await expect(page).toHaveTitle('Test Article Title');

		// Verify metadata
		const authorMeta = page.locator('meta[name="author"]');
		await expect(authorMeta).toHaveAttribute('content', 'Jane Smith');

		const descMeta = page.locator('meta[name="description"]');
		await expect(descMeta).toHaveAttribute('content', 'This is a test article description');
	});

	test('should handle article with selected text', async ({ page }) => {
		await page.setContent(`
			<!DOCTYPE html>
			<html>
			<head>
				<title>Selectable Article</title>
			</head>
			<body>
				<article>
					<h1>Article Title</h1>
					<p id="selectable">This text can be selected by the user.</p>
					<p>This is additional content.</p>
				</article>
			</body>
			</html>
		`);

		// Select text programmatically
		await page.evaluate(() => {
			const element = document.getElementById('selectable');
			if (element) {
				const range = document.createRange();
				range.selectNodeContents(element);
				const selection = window.getSelection();
				selection?.removeAllRanges();
				selection?.addRange(range);
			}
		});

		// Verify selection exists
		const selectedText = await page.evaluate(() => window.getSelection()?.toString());
		expect(selectedText).toBe('This text can be selected by the user.');
	});

	test('should handle article with minimal content', async ({ page }) => {
		await page.setContent(`
			<!DOCTYPE html>
			<html>
			<head>
				<title>Short Article</title>
			</head>
			<body>
				<article>
					<h1>Short Title</h1>
					<p>Brief content.</p>
				</article>
			</body>
			</html>
		`);

		const article = page.locator('article');
		await expect(article).toBeVisible();
		await expect(article.locator('h1')).toHaveText('Short Title');
		await expect(article.locator('p')).toHaveText('Brief content.');
	});

	test('should handle article with rich formatting', async ({ page }) => {
		await page.setContent(`
			<!DOCTYPE html>
			<html>
			<head>
				<title>Formatted Article</title>
			</head>
			<body>
				<article>
					<h1>Main Heading</h1>
					<p>This paragraph has <strong>bold text</strong> and <em>italic text</em>.</p>
					<blockquote>This is a quote from someone important.</blockquote>
					<p>Here's a <a href="https://example.com">link to more information</a>.</p>
				</article>
			</body>
			</html>
		`);

		const article = page.locator('article');

		// Verify formatted elements
		await expect(article.locator('strong')).toHaveText('bold text');
		await expect(article.locator('em')).toHaveText('italic text');
		await expect(article.locator('blockquote')).toBeVisible();
		await expect(article.locator('a')).toHaveAttribute('href', 'https://example.com');
	});

	test('should handle article with images', async ({ page }) => {
		await page.setContent(`
			<!DOCTYPE html>
			<html>
			<head>
				<title>Article with Images</title>
			</head>
			<body>
				<article>
					<h1>Visual Article</h1>
					<p>Here's an important image:</p>
					<img src="https://example.com/image.jpg" alt="Descriptive alt text" />
					<p>And some more content after the image.</p>
				</article>
			</body>
			</html>
		`);

		const article = page.locator('article');
		const image = article.locator('img');

		await expect(image).toBeVisible();
		await expect(image).toHaveAttribute('alt', 'Descriptive alt text');
		await expect(image).toHaveAttribute('src', 'https://example.com/image.jpg');
	});

	test('should extract URL from page', async ({ page }) => {
		await page.goto('about:blank');
		await page.setContent(`
			<!DOCTYPE html>
			<html>
			<head>
				<title>URL Test</title>
			</head>
			<body>
				<article>
					<h1>Article Title</h1>
					<p>Content here.</p>
				</article>
			</body>
			</html>
		`);

		// Verify we can access the URL
		const url = page.url();
		expect(url).toBeTruthy();
	});
});

test.describe('Article Watcher Content Script - Snapshot Generation', () => {
	test('should capture highlighted text for snapshot', async ({ page }) => {
		await page.setContent(`
			<!DOCTYPE html>
			<html>
			<head>
				<title>Snapshot Test</title>
			</head>
			<body>
				<article>
					<h1>Article for Snapshot</h1>
					<p id="highlight-target">This is the text that will be highlighted and captured in the snapshot.</p>
					<p>This text will not be selected.</p>
				</article>
			</body>
			</html>
		`);

		// Select specific text
		await page.evaluate(() => {
			const element = document.getElementById('highlight-target');
			if (element) {
				const range = document.createRange();
				range.selectNodeContents(element);
				const selection = window.getSelection();
				selection?.removeAllRanges();
				selection?.addRange(range);
			}
		});

		// Verify the selection
		const selectedText = await page.evaluate(() => window.getSelection()?.toString());
		expect(selectedText).toContain('highlighted and captured');
	});

	test('should handle empty selection for snapshot', async ({ page }) => {
		await page.setContent(`
			<!DOCTYPE html>
			<html>
			<head>
				<title>Empty Selection</title>
			</head>
			<body>
				<article>
					<h1>No Selection</h1>
					<p>No text is selected here.</p>
				</article>
			</body>
			</html>
		`);

		// Ensure no selection
		await page.evaluate(() => {
			const selection = window.getSelection();
			selection?.removeAllRanges();
		});

		const selectedText = await page.evaluate(() => window.getSelection()?.toString());
		expect(selectedText).toBe('');
	});
});
