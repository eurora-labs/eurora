<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import EyeIcon from '@lucide/svelte/icons/eye';
	import FileTextIcon from '@lucide/svelte/icons/file-text';
	import LockIcon from '@lucide/svelte/icons/lock';
	import ShieldIcon from '@lucide/svelte/icons/shield';
</script>

<div class="container mx-auto max-w-5xl px-4 py-16">
	<div class="mb-12 text-center">
		<h1 class="mb-4 text-4xl font-bold">Privacy Policy</h1>
		<p class="mx-auto max-w-2xl text-xl text-gray-600">
			At Eurora, we take your privacy seriously. Learn how we collect, use, and protect your
			data.
		</p>
	</div>

	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-3">
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<ShieldIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Data Protection</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600">
					Your data is protected with industry-standard encryption both in transit and at
					rest. We implement multiple layers of security to ensure your information
					remains safe.
				</p>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<LockIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Privacy Controls</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600">
					You have full control over what data you share. Our granular privacy settings
					allow you to decide exactly what information Eurora can access and store.
				</p>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex items-center gap-2">
					<EyeIcon class="h-6 w-6 text-purple-600" />
					<Card.Title>Transparency</Card.Title>
				</div>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600">
					We believe in complete transparency about how your data is used. You can request
					a copy of your data at any time, and we'll provide clear information about what
					we collect and why.
				</p>
			</Card.Content>
		</Card.Root>
	</div>

	<Card.Root class="mb-16 p-6">
		<Card.Header>
			<div class="flex items-center gap-2">
				<FileTextIcon class="h-6 w-6 text-purple-600" />
				<Card.Title>Privacy Policy</Card.Title>
			</div>
			<Card.Description>Last updated: March 15, 2025</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="space-y-6 text-gray-700">
				<div>
					<h3 class="mb-2 text-lg font-medium">1. Introduction</h3>
					<p>
						Welcome to Eurora ("we," "our," or "us"). We are committed to protecting
						your privacy and handling your data with transparency and care. This Privacy
						Policy explains how we collect, use, disclose, and safeguard your
						information when you use our application and services.
					</p>
					<p class="mt-2">
						By using Eurora, you agree to the collection and use of information in
						accordance with this policy. We will not use or share your information with
						anyone except as described in this Privacy Policy.
					</p>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">2. Information We Collect</h3>
					<p>We collect several types of information for various purposes:</p>
					<h4 class="mb-1 mt-3 font-medium">2.1 Personal Information</h4>
					<p>
						When you create an account, we may collect personally identifiable
						information, such as:
					</p>
					<ul class="mt-1 list-disc pl-5">
						<li>Email address</li>
						<li>Name</li>
						<li>Profile information you choose to provide</li>
					</ul>

					<h4 class="mb-1 mt-3 font-medium">2.2 Usage Data</h4>
					<p>
						We may collect information on how the application is accessed and used
						("Usage Data"). This may include:
					</p>
					<ul class="mt-1 list-disc pl-5">
						<li>Types of queries you make</li>
						<li>Features you use</li>
						<li>Time spent on the application</li>
						<li>Technical information about your device and connection</li>
					</ul>

					<h4 class="mb-1 mt-3 font-medium">2.3 Thread Data</h4>
					<p>
						To provide our AI services, we process the content of your threads with
						Eurora. This includes:
					</p>
					<ul class="mt-1 list-disc pl-5">
						<li>Text of your queries</li>
						<li>Context you provide</li>
						<li>Our responses</li>
					</ul>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">3. How We Use Your Information</h3>
					<p>We use the collected data for various purposes:</p>
					<ul class="mt-1 list-disc pl-5">
						<li>To provide and maintain our service</li>
						<li>To notify you about changes to our service</li>
						<li>To provide customer support</li>
						<li>To improve our AI models and services</li>
						<li>To detect, prevent, and address technical issues</li>
					</ul>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">4. Data Storage and Security</h3>
					<p>
						We implement a variety of security measures to maintain the safety of your
						personal information:
					</p>
					<ul class="mt-1 list-disc pl-5">
						<li>All data is encrypted in transit and at rest</li>
						<li>Access to personal data is restricted to authorized personnel only</li>
						<li>Regular security audits and assessments</li>
						<li>Compliance with industry-standard security practices</li>
					</ul>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">5. Data Retention</h3>
					<p>
						We will retain your personal information only for as long as is necessary
						for the purposes set out in this Privacy Policy. We will retain and use your
						information to the extent necessary to comply with our legal obligations,
						resolve disputes, and enforce our policies.
					</p>
					<p class="mt-2">
						Usage data is generally retained for a shorter period, except when this data
						is used to strengthen security or improve functionality, or we are legally
						obligated to retain this data for longer periods.
					</p>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">6. Your Data Rights</h3>
					<p>You have the following rights regarding your personal data:</p>
					<ul class="mt-1 list-disc pl-5">
						<li>Right to access your personal data</li>
						<li>Right to request correction of inaccurate data</li>
						<li>Right to request deletion of your data</li>
						<li>Right to restrict or object to processing</li>
						<li>Right to data portability</li>
						<li>Right to withdraw consent at any time</li>
					</ul>
					<p class="mt-2">
						To exercise these rights, please contact us using the information provided
						in the "Contact Us" section.
					</p>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">7. Third-Party Services</h3>
					<p>
						Eurora may use third-party services that collect information used to
						identify you. These third parties have their own privacy policies addressing
						how they use such information.
					</p>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">8. Children's Privacy</h3>
					<p>
						Our service is not intended for use by children under the age of 13. We do
						not knowingly collect personally identifiable information from children
						under 13. If we discover that a child under 13 has provided us with personal
						information, we will delete it immediately.
					</p>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">9. Changes to This Privacy Policy</h3>
					<p>
						We may update our Privacy Policy from time to time. We will notify you of
						any changes by posting the new Privacy Policy on this page and updating the
						"Last updated" date.
					</p>
					<p class="mt-2">
						You are advised to review this Privacy Policy periodically for any changes.
						Changes to this Privacy Policy are effective when they are posted on this
						page.
					</p>
				</div>

				<div>
					<h3 class="mb-2 text-lg font-medium">10. Contact Us</h3>
					<p>
						If you have any questions about this Privacy Policy, please contact us at
						privacy@eurora-labs.com.
					</p>
				</div>
			</div>
		</Card.Content>
	</Card.Root>

	<div class="grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Data Request</Card.Title>
				<Card.Description>Request a copy of your personal data</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-6 text-gray-600">
					Under data protection laws, you have the right to access, correct, update, or
					request deletion of your personal information. You can request a copy of all
					data we have stored about you.
				</p>
				<Button variant="outline" class="w-full">Request My Data</Button>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Privacy Questions</Card.Title>
				<Card.Description>Get answers to your privacy concerns</Card.Description>
			</Card.Header>
			<Card.Content>
				<p class="mb-6 text-gray-600">
					Have questions or concerns about your privacy? Our dedicated privacy team is
					here to help. Contact us for any privacy-related inquiries.
				</p>
				<Button variant="outline" class="w-full">Contact Privacy Team</Button>
			</Card.Content>
		</Card.Root>
	</div>
</div>
