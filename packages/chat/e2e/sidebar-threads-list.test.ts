import { test, expect, type Page } from '@playwright/test';

const TEST_PATH = '/sidebar-threads-list';

async function seed(page: Page, count: number) {
	await page.evaluate((n) => (window as any).__test.seedAndLoad(n), count);
}

async function addThread(page: Page, title: string) {
	await page.evaluate((t) => (window as any).__test.addThread(t), title);
}

async function addUntitledThread(page: Page) {
	await page.evaluate(() => (window as any).__test.addUntitledThread());
}

async function seedWithoutLoad(page: Page, count: number) {
	await page.evaluate((n) => (window as any).__test.seedWithoutLoad(n), count);
}

async function setDeleteFailure(page: Page, shouldFail: boolean) {
	await page.evaluate((f) => (window as any).__test.setDeleteFailure(f), shouldFail);
}

function threadItems(page: Page) {
	return page.locator('[data-sidebar="menu-item"]');
}

function threadButton(page: Page, name: string) {
	return page.locator('[data-sidebar="menu-button"]', { hasText: name });
}

async function openDeleteDialog(page: Page, index = 0) {
	const item = threadItems(page).nth(index);
	await item.hover();
	await item.locator('[data-sidebar="menu-action"]').click();
	await page.locator('[data-slot="dropdown-menu-item"]', { hasText: 'Delete' }).click();
}

async function confirmDelete(page: Page) {
	await page
		.locator('[data-slot="dialog-content"]')
		.getByRole('button', { name: 'Delete' })
		.click();
}

test.describe('SidebarThreadsList', () => {
	test.beforeEach(async ({ page }) => {
		await page.goto(TEST_PATH);
		await page.waitForSelector('[data-testid="debug-panel"]');
	});

	test.describe('empty state', () => {
		test('shows empty state when no threads exist', async ({ page }) => {
			await seed(page, 0);
			await expect(page.locator('[data-slot="empty"]')).toBeVisible();
			await expect(page.getByText('No chats yet')).toBeVisible();
		});

		test('empty state disappears when threads are added', async ({ page }) => {
			await seed(page, 0);
			await expect(page.locator('[data-slot="empty"]')).toBeVisible();
			await seed(page, 3);
			await expect(page.locator('[data-slot="empty"]')).not.toBeVisible();
			await expect(threadItems(page)).toHaveCount(3);
		});
	});

	test.describe('thread listing', () => {
		test('displays all seeded threads', async ({ page }) => {
			await seed(page, 5);
			await expect(threadItems(page)).toHaveCount(5);
			for (let i = 1; i <= 5; i++) {
				await expect(threadButton(page, `Chat ${i}`)).toBeVisible();
			}
		});

		test('displays threads with correct titles', async ({ page }) => {
			await addThread(page, 'Alpha');
			await addThread(page, 'Beta');
			await expect(threadButton(page, 'Alpha')).toBeVisible();
			await expect(threadButton(page, 'Beta')).toBeVisible();
		});

		test('threads without title show "New Thread"', async ({ page }) => {
			await addUntitledThread(page);
			await expect(threadButton(page, 'New Thread')).toBeVisible();
		});
	});

	test.describe('thread selection', () => {
		test('clicking a thread selects it', async ({ page }) => {
			await seed(page, 3);
			await threadButton(page, 'Chat 1').click();
			await expect(page.getByTestId('selected-thread-id')).not.toHaveText('');
			await expect(page.getByTestId('active-thread-id')).not.toHaveText('');
		});

		test('selected thread gets active state', async ({ page }) => {
			await seed(page, 3);
			const btn = threadButton(page, 'Chat 2');
			await btn.click();

			await expect(btn).toHaveAttribute('data-active', 'true');
		});

		test('selecting a different thread deactivates the previous one', async ({ page }) => {
			await seed(page, 3);
			const btn1 = threadButton(page, 'Chat 1');
			const btn2 = threadButton(page, 'Chat 2');

			await btn1.click();
			await expect(btn1).toHaveAttribute('data-active', 'true');

			await btn2.click();
			await expect(btn2).toHaveAttribute('data-active', 'true');
			await expect(btn1).toHaveAttribute('data-active', 'false');
		});

		test('onThreadSelect callback fires with the thread id', async ({ page }) => {
			await seed(page, 2);
			await threadButton(page, 'Chat 1').click();
			const lastAction = await page.getByTestId('last-action').textContent();
			expect(lastAction).toContain('selected:');
		});
	});

	test.describe('thread deletion', () => {
		test('opens confirmation dialog from context menu', async ({ page }) => {
			await seed(page, 3);
			await openDeleteDialog(page);

			await expect(page.locator('[data-slot="dialog-content"]')).toBeVisible();
			await expect(page.getByText('Delete Chat')).toBeVisible();
		});

		test('dialog shows the thread title', async ({ page }) => {
			await seed(page, 3);
			await openDeleteDialog(page);

			await expect(page.locator('[data-slot="dialog-description"]')).toContainText('Chat 1');
		});

		test('cancel closes dialog without deleting', async ({ page }) => {
			await seed(page, 3);
			await openDeleteDialog(page);

			await page.locator('[data-slot="dialog-close"]').click();

			await expect(page.locator('[data-slot="dialog-content"]')).not.toBeVisible();
			await expect(threadItems(page)).toHaveCount(3);
		});

		test('confirming delete removes thread from list', async ({ page }) => {
			await seed(page, 3);
			await openDeleteDialog(page);
			await confirmDelete(page);

			await expect(threadItems(page)).toHaveCount(2);
			await expect(page.getByTestId('thread-count')).toHaveText('2');
		});

		test('deleting the active thread clears selection', async ({ page }) => {
			await seed(page, 3);

			await threadButton(page, 'Chat 1').click();
			await expect(page.getByTestId('active-thread-id')).not.toHaveText('');

			await openDeleteDialog(page);
			await confirmDelete(page);

			await expect(page.getByTestId('active-thread-id')).toHaveText('');
		});

		test('deleting a non-active thread preserves selection', async ({ page }) => {
			await seed(page, 3);

			await threadButton(page, 'Chat 2').click();
			const activeId = await page.getByTestId('active-thread-id').textContent();

			await openDeleteDialog(page);
			await confirmDelete(page);

			await expect(threadItems(page)).toHaveCount(2);
			await expect(page.getByTestId('active-thread-id')).toHaveText(activeId!);
		});

		test('failed delete shows error toast', async ({ page }) => {
			await seed(page, 3);
			await setDeleteFailure(page, true);

			await openDeleteDialog(page);
			await confirmDelete(page);

			await expect(page.getByText('Failed to delete chat')).toBeVisible();
			await expect(threadItems(page)).toHaveCount(3);
		});

		test('can delete multiple threads sequentially', async ({ page }) => {
			await seed(page, 5);

			for (let remaining = 5; remaining > 3; remaining--) {
				await openDeleteDialog(page);
				await confirmDelete(page);
				await expect(threadItems(page)).toHaveCount(remaining - 1);
			}

			await expect(threadItems(page)).toHaveCount(3);
		});

		test('deleting all threads shows empty state', async ({ page }) => {
			await seed(page, 2);

			for (let i = 0; i < 2; i++) {
				await openDeleteDialog(page);
				await confirmDelete(page);
				if (i === 0) await expect(threadItems(page)).toHaveCount(1);
			}

			await expect(page.locator('[data-slot="empty"]')).toBeVisible();
			await expect(page.getByText('No chats yet')).toBeVisible();
		});
	});

	test.describe('infinite scroll', () => {
		test('loads more threads when scrolling to bottom', async ({ page }) => {
			await seedWithoutLoad(page, 40);

			const sidebar = page.locator('[data-sidebar="content"]');
			await sidebar.evaluate((el) => el.scrollTo(0, el.scrollHeight));

			await expect(threadItems(page)).toHaveCount(40, { timeout: 5000 });
			await expect(page.getByTestId('thread-count')).toHaveText('40');
		});
	});
});
