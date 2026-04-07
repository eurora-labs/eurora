import { test, expect, type Page } from '@playwright/test';

const TEST_PATH = '/chat-prompt-input';

const textarea = (page: Page) => page.locator('textarea[name="message"]');
const submitBtn = (page: Page) => page.locator('[data-slot="prompt-input-submit"]');
const suggestions = (page: Page) => page.locator('[data-slot="suggestion"]');

test.describe('ChatPromptInput', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto(TEST_PATH);
		await page.waitForSelector('[data-testid="debug-panel"]');
	});

	test.describe('textarea', () => {
		test('renders with custom placeholder', async ({ page }) => {
			await expect(textarea(page)).toHaveAttribute('placeholder', 'Ask me anything...');
		});

		test('accepts text input', async ({ page }) => {
			await textarea(page).fill('Hello world');
			await expect(textarea(page)).toHaveValue('Hello world');
		});
	});

	test.describe('submit', () => {
		test('submits via Enter key', async ({ page }) => {
			await textarea(page).fill('Test message');
			await textarea(page).press('Enter');

			await expect(page.getByTestId('last-submitted')).toHaveText('Test message');
			await expect(page.getByTestId('submit-count')).toHaveText('1');
		});

		test('submits via submit button', async ({ page }) => {
			await textarea(page).fill('Button submit');
			await submitBtn(page).click();

			await expect(page.getByTestId('last-submitted')).toHaveText('Button submit');
		});

		test('does not submit empty text', async ({ page }) => {
			await textarea(page).press('Enter');

			await expect(page.getByTestId('submit-count')).toHaveText('0');
		});

		test('does not submit whitespace-only text', async ({ page }) => {
			await textarea(page).fill('   ');
			await textarea(page).press('Enter');

			await expect(page.getByTestId('submit-count')).toHaveText('0');
		});

		test('Shift+Enter does not submit (inserts newline)', async ({ page }) => {
			await textarea(page).fill('Line one');
			await textarea(page).press('Shift+Enter');

			await expect(page.getByTestId('submit-count')).toHaveText('0');
		});

		test('can submit multiple messages sequentially', async ({ page }) => {
			await textarea(page).fill('First');
			await textarea(page).press('Enter');
			await expect(page.getByTestId('submit-count')).toHaveText('1');

			await textarea(page).fill('Second');
			await textarea(page).press('Enter');
			await expect(page.getByTestId('submit-count')).toHaveText('2');
			await expect(page.getByTestId('last-submitted')).toHaveText('Second');
		});
	});

	test.describe('suggestions', () => {
		test('shows suggestions when thread has no messages', async ({ page }) => {
			await page.evaluate(() => (window as any).__test.setActiveThreadEmpty('thread-1'));
			await expect(suggestions(page)).toHaveCount(3);
			await expect(suggestions(page).first()).toContainText('Tell me a joke');
		});

		test('hides suggestions when thread has messages', async ({ page }) => {
			await page.evaluate(() =>
				(window as any).__test.setActiveThreadWithMessages('thread-1'),
			);
			await expect(suggestions(page)).toHaveCount(0);
		});

		test('clicking a suggestion submits its text', async ({ page }) => {
			await page.evaluate(() => (window as any).__test.setActiveThreadEmpty('thread-1'));

			await suggestions(page).filter({ hasText: 'Write a poem' }).click();

			await expect(page.getByTestId('last-submitted')).toHaveText('Write a poem');
		});

		test('hides suggestions when no active thread', async ({ page }) => {
			await expect(suggestions(page)).toHaveCount(0);
		});
	});

	test.describe('streaming state', () => {
		test('shows stop button during streaming', async ({ page }) => {
			await page.evaluate(() => {
				const t = (window as any).__test;
				t.setActiveThreadWithMessages('thread-1');
			});
			await page.evaluate(() => (window as any).__test.simulateStreaming('thread-1'));

			await expect(submitBtn(page)).toHaveAttribute('aria-label', 'Stop');
		});

		test('shows submit button when not streaming', async ({ page }) => {
			await expect(submitBtn(page)).toHaveAttribute('aria-label', 'Submit');
		});
	});
});
