<script lang="ts">
	import { Badge } from '@eurora/ui/components/badge/index';
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import * as Table from '@eurora/ui/components/table/index';
	// Placeholder — replace with real subscription state
	const hasPaidPlan = false;

	const invoices = [
		{ id: 'INV-001', date: 'Jan 10, 2026', amount: '$0.00', status: 'Paid', plan: 'Free' },
		{ id: 'INV-002', date: 'Dec 10, 2025', amount: '$0.00', status: 'Paid', plan: 'Free' },
		{ id: 'INV-003', date: 'Nov 10, 2025', amount: '$0.00', status: 'Paid', plan: 'Free' },
	];
</script>

<svelte:head>
	<title>Billing - Eurora Labs</title>
</svelte:head>

<div class="space-y-8">
	<div class="flex items-center justify-between py-2">
		<div>
			<h3 class="text-2xl font-bold tracking-tight">Free</h3>
			<p class="mt-0.5 text-sm text-muted-foreground">$0.00 / month</p>
		</div>
		{#if hasPaidPlan}
			<Button variant="outline" size="sm">Update payment method</Button>
		{:else}
			<Button size="sm">Upgrade</Button>
		{/if}
	</div>

	<div>
		<h3 class="mb-4 text-base font-semibold">Invoice History</h3>
		<Card.Root class="p-2 border-none">
			<Table.Root>
				<Table.Header>
					<Table.Row class="border-none">
						<Table.Head>Invoice</Table.Head>
						<Table.Head>Date</Table.Head>
						<Table.Head>Plan</Table.Head>
						<Table.Head>Amount</Table.Head>
						<Table.Head class="text-right">Status</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each invoices as invoice (invoice.id)}
						<Table.Row class="border-none">
							<Table.Cell class="font-medium">{invoice.id}</Table.Cell>
							<Table.Cell>{invoice.date}</Table.Cell>
							<Table.Cell>{invoice.plan}</Table.Cell>
							<Table.Cell>{invoice.amount}</Table.Cell>
							<Table.Cell class="text-right">
								<Badge variant="secondary">{invoice.status}</Badge>
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</Card.Root>
	</div>
</div>
