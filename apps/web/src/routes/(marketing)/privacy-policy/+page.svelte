<script lang="ts">
	import * as Card from '@eurora/ui/components/card/index';
	import FileTextIcon from '@lucide/svelte/icons/file-text';
</script>

<div class="container mx-auto max-w-4xl px-4 py-16">
	<div class="mb-12 text-center">
		<h1 class="mb-2 text-4xl font-bold">Privacy Policy</h1>
		<p class="mb-4 text-sm">Last updated: February 24, 2026</p>
	</div>

	<Card.Root class="p-6">
		<Card.Header>
			<div class="flex items-center gap-2">
				<FileTextIcon class="h-6 w-6" />
				<Card.Title>Privacy Policy</Card.Title>
			</div>
		</Card.Header>
		<Card.Content>
			<div class="prose prose-gray max-w-none space-y-6">
				<section>
					<p>
						This is the Privacy Policy of Eurora Labs B.V. (hereinafter referred to as
						"Eurora Labs B.V.," "we," "us" or "our"), a company having its address at
						Braillelaan 65, 5252 CW Vlijmen. Eurora Labs B.V. is registered with the
						Chamber of Commerce under number 98516167.
					</p>
					<p class="mt-4">
						This Privacy Policy explains how we collect, use, share and protect
						information in relation to our website www.eurora-labs.com (the "Website"),
						our desktop application (the "Desktop Application"), our browser extension
						(the "Browser Extension"), and any related services (collectively, the
						"Service"). We process personal data in a manner that is in accordance with
						the General Data Protection Regulation (the "GDPR"), the GDPR-based
						legislation and the other current privacy legislation.
					</p>
					<p class="mt-4">
						By using the Service, you understand and agree to the collection and use of
						information in accordance with this Privacy Policy. Our Privacy Policy
						applies to all visitors, users, and others who access the Service ("Users").
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">
						1. What kinds of information do we collect?
					</h2>

					<h3 class="mb-2 text-lg font-semibold">1.1 Account information</h3>
					<p>We collect the following information you provide us with directly:</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							Your username, display name, password and e-mail address when you
							register for an account;
						</li>
						<li>
							Authentication data when you sign in via a third-party provider (Google
							or GitHub), including your name, e-mail address and provider-specific
							user identifier;
						</li>
						<li>
							Communications between you and Eurora Labs B.V. (e.g., Service-related
							emails);
						</li>
					</ul>

					<h3 class="mb-2 mt-6 text-lg font-semibold">1.2 Desktop Application data</h3>
					<p>
						The Desktop Application is an AI-powered productivity assistant. It
						continuously collects contextual information about your current activity in
						the background and stores it locally on your device. This data is
						<strong>sent to our servers only when you actively submit a query</strong> and
						includes:
					</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							<strong>Active window information:</strong> the name of the application you
							are currently using, the window title, and the application icon;
						</li>
						<li>
							<strong>Activity timestamps:</strong> when an application was focused and
							for how long;
						</li>
						<li>
							<strong>Conversation data:</strong> your queries to the AI assistant and the
							responses generated;
						</li>
					</ul>

					<h3 class="mb-2 mt-6 text-lg font-semibold">1.3 Browser Extension data</h3>
					<p>
						The Browser Extension enhances the Desktop Application by providing web-page
						context when you submit a query. The Browser Extension does not connect to
						any third-party services; all data is transmitted exclusively to the Desktop
						Application on your local device via native messaging. When you submit a
						query, the following data may be collected depending on the website you are
						viewing:
					</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							<strong>General web pages:</strong> page URL, page title, favicon, and article
							text content extracted from the page;
						</li>
						<li>
							<strong>YouTube:</strong> video URL, video title, video transcript (including
							auto-generated captions), current playback time, and a screenshot of the current
							video frame;
						</li>
						<li>
							<strong>Twitter/X:</strong> page URL, page title, the text content of tweets
							visible on the page, tweet author names and timestamps;
						</li>
						<li>
							<strong>Selected text:</strong> any text you have highlighted on the page;
						</li>
					</ul>

					<h3 class="mb-2 mt-6 text-lg font-semibold">
						1.4 Payment and subscription data
					</h3>
					<p>
						When you subscribe to a paid plan, we collect and process payment
						information through our payment processor, Stripe. We store your Stripe
						customer identifier, e-mail address, subscription status, billing period,
						and invoice records. We do not store your full credit card number or bank
						account details directly; these are handled by Stripe in accordance with
						their own privacy policy.
					</p>

					<h3 class="mb-2 mt-6 text-lg font-semibold">1.5 Log file information</h3>
					<p>
						We collect information that your browser sends whenever you visit our
						Website. This log file information may include information such as your
						computer's Internet Protocol address, browser type, browser version, the
						pages of our Website that you visit, the time and date of your visit, the
						time spent on those pages and other statistics.
					</p>

					<h3 class="mb-2 mt-6 text-lg font-semibold">
						1.6 Analytics and error reporting
					</h3>
					<p>
						We use the following third-party tools to help us operate and improve the
						Service:
					</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							<strong>Google Analytics</strong> (Website): collects anonymous information
							about Website traffic and usage trends using cookies. Cookies are text files
							placed on your device to collect log and behaviour information in anonymous
							form. Google's ability to use and share information collected by Google Analytics
							is restricted by the Google Analytics Terms of Use and the Google Privacy
							Policy. You can prevent Google Analytics from recognising you on return visits
							by disabling cookies in your browser.
						</li>
						<li>
							<strong>PostHog</strong> (backend, EU-hosted): collects anonymous, aggregated
							metrics about Service usage, such as the number of downloads, updates and
							payments. No personally identifiable information is sent to PostHog.
						</li>
						<li>
							<strong>Sentry</strong> (backend): collects error and crash reports to help
							us diagnose and fix technical issues. Sentry does not receive personally identifiable
							information by default.
						</li>
					</ul>

					<h3 class="mb-2 mt-6 text-lg font-semibold">
						1.7 Telemetry in the Desktop Application
					</h3>
					<p>
						The Desktop Application may collect anonymous usage metrics and error
						reports. During initial setup and at any time via the settings, you can
						enable or disable: anonymous usage metrics, anonymous error reports, and
						non-anonymous usage metrics. Anonymous metrics and anonymous error reports
						are enabled by default. You may opt out at any time through the application
						settings.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">2. How do we use this information?</h2>
					<p>We use the information we collect for the following purposes:</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							To provide and operate the Service, including processing your queries
							through the AI assistant with relevant context from your current
							activity;
						</li>
						<li>
							To manage your account, authenticate your identity and process payments;
						</li>
						<li>To remember your preferences so you will not have to re-enter them;</li>
						<li>
							To provide, improve, test and monitor the effectiveness of the Service;
						</li>
						<li>
							To monitor aggregated metrics such as total number of users, traffic and
							usage patterns;
						</li>
						<li>To diagnose and fix technical problems;</li>
						<li>To develop and test new products and features;</li>
						<li>
							To enforce token usage limits associated with your subscription plan;
						</li>
					</ul>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">3. AI processing</h2>
					<p>
						When you submit a query in the Desktop Application, the contextual data
						described in Sections 1.2 and 1.3 is sent to our backend and processed by a
						third-party AI inference provider, Nebul, which operates a sovereign cloud
						platform hosted in Europe. Nebul maintains a no-logging policy and processes
						data anonymously. Your data is not stored by Nebul beyond the duration of
						the inference request.
					</p>
					<p class="mt-4">
						We record the number of AI tokens consumed per query for the purpose of
						enforcing subscription plan limits. Your conversation history (queries and
						AI responses) is stored on our servers so that you can access it across
						sessions. Activity data (application names, window titles and focus
						timestamps) is also stored on our servers to provide contextual history for
						your queries.
					</p>
					<p class="mt-4">
						We do not use your data to train, fine-tune or improve AI models. Your
						queries and contextual data are processed solely to generate responses to
						your requests.
					</p>
					<p class="mt-4">
						The Desktop Application also offers a local-only mode, in which you can
						connect a local AI model (e.g., via Ollama). In local-only mode, no
						conversation data, activity data or contextual data is sent to our servers
						or any third party.
					</p>
					<p class="mt-4">
						You may optionally configure a direct connection to third-party AI providers
						such as OpenAI or Anthropic using your own API key. In that case, data is
						sent directly from the Desktop Application on your device to the provider
						you configured, without passing through our servers. We are not responsible
						for the data processing practices of those providers; please refer to their
						respective privacy policies.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">4. Legal basis for processing data</h2>
					<p>
						Eurora Labs B.V. processes personal data on the following legal bases under
						the GDPR:
					</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							<strong>Performance of a contract</strong> (Article 6(1)(b) GDPR): to provide
							the Service you have signed up for, including processing queries and managing
							your account;
						</li>
						<li>
							<strong>Legitimate interest</strong> (Article 6(1)(f) GDPR): to improve and
							secure the Service, perform analytics and diagnose technical issues;
						</li>
						<li>
							<strong>Consent</strong> (Article 6(1)(a) GDPR): for non-anonymous telemetry,
							marketing communications and cookies;
						</li>
						<li>
							<strong>Legal obligation</strong> (Article 6(1)(c) GDPR): to comply with applicable
							laws and regulations;
						</li>
					</ul>
					<p class="mt-4">
						We process your personal data only to the extent necessary to achieve the
						purposes for which it was collected. We do not use your personal data for
						automated decision-making or profiling that produces legal or similarly
						significant effects.
					</p>
					<p class="mt-4">
						Providing the personal data described in Sections 1.1 and 1.2 is necessary
						for us to provide the Service. If you choose not to provide this data, we
						may not be able to offer you all or part of the Service's functionality.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">5. Communications</h2>
					<p>
						We may use your information to contact you with newsletters, marketing or
						promotional materials and other information that may be of interest to you.
						You may opt out of receiving any, or all, of these communications from us by
						following the unsubscribe link.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">6. How is this information shared?</h2>
					<p>
						We will not rent or sell your information to third parties. We share data
						with the following categories of recipients only as necessary to operate the
						Service:
					</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							<strong>Nebul</strong> (AI inference provider, EU): processes your queries
							and contextual data for AI responses under a no-logging policy;
						</li>
						<li>
							<strong>Stripe</strong> (payment processor): processes payment transactions
							and subscription management;
						</li>
						<li>
							<strong>Google / GitHub</strong> (authentication providers): facilitate account
							sign-in when you choose to use OAuth;
						</li>
						<li>
							<strong>Google Analytics</strong> (Website analytics): anonymous website traffic
							analysis;
						</li>
						<li>
							<strong>PostHog</strong> (backend analytics, EU-hosted): anonymous, aggregated
							usage metrics;
						</li>
						<li>
							<strong>Sentry</strong> (error tracking): anonymous error and crash reports;
						</li>
						<li>
							<strong>Amazon Web Services</strong> (storage): certain data is stored on
							AWS S3 in encrypted form (encrypted prior to upload on our European infrastructure);
						</li>
					</ul>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">
						7. Data storage, security and international transfers
					</h2>
					<p>
						Your data is primarily hosted in Europe on Nebul, a sovereign European cloud
						platform. Certain data may be stored on Amazon Web Services (AWS) S3;
						however, such data is encrypted on our European infrastructure using
						XChaCha20-Poly1305 authenticated encryption before being transferred to AWS.
					</p>
					<p class="mt-4">
						Eurora Labs B.V. has taken appropriate technical and organisational measures
						to protect your information against loss or unlawful processing, including:
					</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							Encryption of data at rest using XChaCha20-Poly1305 authenticated
							encryption;
						</li>
						<li>Passwords hashed with Argon2;</li>
						<li>OAuth tokens encrypted before database storage;</li>
						<li>Authenticated and encrypted communication channels (gRPC with TLS);</li>
						<li>
							Local secrets stored in your operating system's keychain or in encrypted
							files on your device;
						</li>
					</ul>
					<p class="mt-4">
						However, Eurora Labs B.V. cannot guarantee absolute security of any
						information you transmit. You are responsible for maintaining the secrecy of
						your password and account information.
					</p>
					<p class="mt-4">
						Where data is transferred outside the European Economic Area, we ensure
						appropriate safeguards are in place in accordance with GDPR requirements,
						including the use of encryption prior to transfer.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">8. Local data storage</h2>
					<p>
						The Desktop Application stores certain data locally on your device,
						including:
					</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>Application settings and preferences;</li>
						<li>
							Authentication tokens and API keys, encrypted using XChaCha20-Poly1305
							with a master key stored in your operating system's keychain;
						</li>
						<li>
							A local activity database for caching recent activity context. When you
							submit a query, this data is also transmitted to and stored on our
							servers (see Section 3);
						</li>
					</ul>
					<p class="mt-4">
						In local-only mode, all data remains exclusively on your device and is not
						transmitted to our servers.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">9. Data retention</h2>
					<p>
						Your data is retained for as long as your account is active. You may delete
						specific data (such as individual conversations or activities) through the
						Service, or delete your entire account, at which point all associated data
						will be removed from our servers. We may retain certain data for a limited
						period where required by law or to resolve disputes.
					</p>
					<p class="mt-4">
						Temporary authentication data is automatically cleaned up: OAuth state
						within one hour, login tokens within one day, and refresh tokens within
						thirty days or upon revocation.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">10. Your rights under the GDPR</h2>
					<p>
						If you are located in the European Economic Area, you have the following
						rights regarding your personal data:
					</p>
					<ul class="mt-2 list-disc space-y-1 pl-5">
						<li>
							<strong>Right of access:</strong> you may request a copy of the personal data
							we hold about you;
						</li>
						<li>
							<strong>Right to rectification:</strong> you may request correction of inaccurate
							or incomplete data;
						</li>
						<li>
							<strong>Right to erasure:</strong> you may request deletion of your personal
							data;
						</li>
						<li>
							<strong>Right to restriction of processing:</strong> you may request that
							we limit how we use your data;
						</li>
						<li>
							<strong>Right to data portability:</strong> you may request your data in a
							structured, commonly used, machine-readable format;
						</li>
						<li>
							<strong>Right to object:</strong> you may object to processing based on legitimate
							interests;
						</li>
						<li>
							<strong>Right to withdraw consent:</strong> where processing is based on consent,
							you may withdraw it at any time;
						</li>
						<li>
							<strong>Right to lodge a complaint:</strong> you have the right to lodge a
							complaint with a supervisory authority, in particular in the EU Member State
							of your habitual residence, place of work or place of the alleged infringement,
							if you consider that the processing of your personal data infringes the GDPR;
						</li>
					</ul>
					<p class="mt-4">
						To exercise any of these rights, please contact us at
						<a
							href="mailto:contact@eurora-labs.com"
							class="text-blue-600 hover:underline">contact@eurora-labs.com</a
						>. We will respond within one month of receiving your request.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">11. Change of control</h2>
					<p>
						If we sell or otherwise transfer part or the whole of Eurora Labs B.V. or
						our assets to another organisation (e.g., in the course of a merger,
						acquisition, bankruptcy, dissolution or liquidation), your information
						collected through the Service may be among the items sold or transferred.
						The buyer or transferee will have to honour the commitments we have made in
						this Privacy Policy.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">12. Legal requests and preventing harm</h2>
					<p>
						We may access, preserve and share your information in response to a legal
						request (such as a search warrant, court order or subpoena) if we have a
						good faith belief that the law requires us to do so. We may also access,
						preserve and share information when we have a good faith belief it is
						necessary to: detect, prevent and address fraud and other illegal activity;
						to protect ourselves, you and others, including as part of investigations;
						and to prevent death or imminent bodily harm.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">
						13. Third-party applications, websites and services
					</h2>
					<p>
						The Service may contain links to third-party applications, websites or
						services. We are not responsible for the privacy practices of those third
						parties. When you navigate to a third-party service, this Privacy Policy no
						longer applies, and we encourage you to review their respective privacy
						policies.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">14. Children's privacy</h2>
					<p>
						The Service is not directed at anyone under the age of 16 ("Children"). We
						do not knowingly collect personally identifiable information from children
						under 16. If you are a parent or guardian and you become aware that your
						child has provided us with personal information, please contact us. If we
						become aware that we have collected personal information from a child under
						16 without verification of parental consent, we will take steps to remove
						that information from our servers.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">15. Changes to this privacy policy</h2>
					<p>
						Eurora Labs B.V. may modify or update this Privacy Policy from time to time.
						We will notify you of any changes by posting the new Privacy Policy on this
						page and updating the "Last updated" date. You are advised to review this
						Privacy Policy periodically. Changes are effective when they are posted on
						this page.
					</p>
				</section>

				<section>
					<h2 class="mb-4 text-2xl font-bold">16. How to contact us</h2>
					<p>
						If you have any questions about this Privacy Policy, please contact us at
						<a
							href="mailto:contact@eurora-labs.com"
							class="text-blue-600 hover:underline">contact@eurora-labs.com</a
						>.
					</p>
					<p class="mt-4">
						Eurora Labs B.V.<br />
						Braillelaan 65<br />
						5252 CW Vlijmen<br />
						The Netherlands<br />
						Chamber of Commerce: 98516167
					</p>
					<p class="mt-4">
						Eurora Labs B.V. has not appointed a Data Protection Officer. For all
						privacy-related inquiries, please use the contact details above.
					</p>
				</section>
			</div>
		</Card.Content>
	</Card.Root>
</div>
