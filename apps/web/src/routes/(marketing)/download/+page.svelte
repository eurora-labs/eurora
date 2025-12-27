<script lang="ts">
	import { Button } from '@eurora/ui/components/button/index';
	import * as Card from '@eurora/ui/components/card/index';
	import DownloadIcon from '@lucide/svelte/icons/download';
	import GlobeIcon from '@lucide/svelte/icons/globe';
	import LaptopIcon from '@lucide/svelte/icons/laptop';
	import MonitorIcon from '@lucide/svelte/icons/monitor';
</script>

<div class="container mx-auto max-w-5xl px-4 py-16">
	<div class="mb-12 text-center">
		<h1 class="mb-4 text-4xl font-bold">Download Eurora</h1>
		<p class="mx-auto max-w-2xl text-xl text-gray-600">
			Get Eurora for your preferred platform and start experiencing AI on your own terms.
		</p>
	</div>

	<div class="mb-16 grid grid-cols-1 gap-8 md:grid-cols-2">
		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Desktop Applications</Card.Title>
				<Card.Description
					>Download Eurora for your desktop operating system.</Card.Description
				>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div
						class="flex items-center justify-between rounded-md border p-4 hover:bg-gray-50"
					>
						<div class="flex items-center gap-3">
							<MonitorIcon class="h-8 w-8 text-blue-600" />
							<div>
								<h3 class="font-medium">Windows</h3>
								<p class="text-sm text-gray-600">Windows 10 or later</p>
							</div>
						</div>
						<Button>
							<DownloadIcon class="mr-2 h-4 w-4" />
							Download
						</Button>
					</div>

					<div
						class="flex items-center justify-between rounded-md border p-4 hover:bg-gray-50"
					>
						<div class="flex items-center gap-3">
							<LaptopIcon class="h-8 w-8 text-gray-800" />
							<div>
								<h3 class="font-medium">macOS</h3>
								<p class="text-sm text-gray-600">macOS 11 or later</p>
							</div>
						</div>
						<Button>
							<DownloadIcon class="mr-2 h-4 w-4" />
							Download
						</Button>
					</div>

					<div
						class="flex items-center justify-between rounded-md border p-4 hover:bg-gray-50"
					>
						<div class="flex items-center gap-3">
							<MonitorIcon class="h-8 w-8 text-orange-600" />
							<div>
								<h3 class="font-medium">Linux</h3>
								<p class="text-sm text-gray-600">Ubuntu, Debian, Fedora</p>
							</div>
						</div>
						<Button>
							<DownloadIcon class="mr-2 h-4 w-4" />
							Download
						</Button>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Browser Extensions</Card.Title>
				<Card.Description>Add Eurora to your favorite browser.</Card.Description>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div
						class="flex items-center justify-between rounded-md border p-4 hover:bg-gray-50"
					>
						<div class="flex items-center gap-3">
							<GlobeIcon class="h-8 w-8 text-blue-500" />
							<div>
								<h3 class="font-medium">Chrome</h3>
								<p class="text-sm text-gray-600">Chrome Web Store</p>
							</div>
						</div>
						<Button>
							<DownloadIcon class="mr-2 h-4 w-4" />
							Add to Chrome
						</Button>
					</div>

					<div
						class="flex items-center justify-between rounded-md border p-4 hover:bg-gray-50"
					>
						<div class="flex items-center gap-3">
							<svg
								class="h-8 w-8 text-blue-600"
								viewBox="0 0 24 24"
								fill="currentColor"
							>
								<path
									d="M12 0C5.372 0 0 5.372 0 12s5.372 12 12 12 12-5.372 12-12S18.628 0 12 0zm7.369 18.769C17.728 20.187 14.954 21 12 21s-5.728-.813-7.369-2.231a9.954 9.954 0 01-2.385-3.647A9.954 9.954 0 013 12c0-1.087.175-2.134.496-3.113a9.897 9.897 0 012.385-3.647C7.522 3.813 10.296 3 12 3s4.478.813 6.119 2.24a9.897 9.897 0 012.385 3.647A9.954 9.954 0 0121 12a9.954 9.954 0 01-.496 3.122 9.954 9.954 0 01-2.385 3.647zM12 4.8a7.2 7.2 0 100 14.4 7.2 7.2 0 000-14.4zm0 12.6a5.4 5.4 0 110-10.8 5.4 5.4 0 010 10.8z"
								/>
							</svg>
							<div>
								<h3 class="font-medium">Firefox</h3>
								<p class="text-sm text-gray-600">Firefox Add-ons</p>
							</div>
						</div>
						<Button>
							<DownloadIcon class="mr-2 h-4 w-4" />
							Add to Firefox
						</Button>
					</div>

					<div
						class="flex items-center justify-between rounded-md border p-4 hover:bg-gray-50"
					>
						<div class="flex items-center gap-3">
							<svg
								class="h-8 w-8 text-blue-800"
								viewBox="0 0 24 24"
								fill="currentColor"
							>
								<path
									d="M12 0C5.373 0 0 5.373 0 12s5.373 12 12 12 12-5.373 12-12S18.627 0 12 0zm0 2.824a9.176 9.176 0 110 18.352 9.176 9.176 0 010-18.352zm-1.176 2.824v6.353h-2.353l3.529 5.294 3.529-5.294h-2.353V5.647h-2.353z"
								/>
							</svg>
							<div>
								<h3 class="font-medium">Edge</h3>
								<p class="text-sm text-gray-600">Microsoft Store</p>
							</div>
						</div>
						<Button>
							<DownloadIcon class="mr-2 h-4 w-4" />
							Add to Edge
						</Button>
					</div>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<Card.Root class="mb-16 p-6">
		<Card.Header>
			<Card.Title>System Requirements</Card.Title>
			<Card.Description
				>Make sure your system meets these requirements before installing</Card.Description
			>
		</Card.Header>
		<Card.Content>
			<div class="grid grid-cols-1 gap-6 md:grid-cols-3">
				<div class="space-y-2">
					<h3 class="text-lg font-medium">Windows</h3>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Windows 10 or later (64-bit)</li>
						<li>4GB RAM minimum</li>
						<li>1GB free disk space</li>
						<li>Intel Core i3 or equivalent</li>
					</ul>
				</div>
				<div class="space-y-2">
					<h3 class="text-lg font-medium">macOS</h3>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>macOS 11 (Big Sur) or later</li>
						<li>4GB RAM minimum</li>
						<li>1GB free disk space</li>
						<li>Intel or Apple Silicon processor</li>
					</ul>
				</div>
				<div class="space-y-2">
					<h3 class="text-lg font-medium">Linux</h3>
					<ul class="list-disc space-y-1 pl-5 text-gray-600">
						<li>Ubuntu 20.04, Debian 10, or Fedora 34+</li>
						<li>4GB RAM minimum</li>
						<li>1GB free disk space</li>
						<li>Intel Core i3 or equivalent</li>
					</ul>
				</div>
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root class="p-6">
		<Card.Header>
			<Card.Title>Installation Instructions</Card.Title>
			<Card.Description>Follow these steps to install Eurora on your device</Card.Description>
		</Card.Header>
		<Card.Content>
			<div class="space-y-6">
				<div>
					<h3 class="mb-2 text-lg font-medium">Windows Installation</h3>
					<ol class="list-decimal space-y-1 pl-5 text-gray-600">
						<li>Download the Windows installer (.exe) file</li>
						<li>Double-click the downloaded file to start the installation</li>
						<li>Follow the on-screen instructions</li>
						<li>Launch Eurora from the Start menu or desktop shortcut</li>
					</ol>
				</div>
				<div>
					<h3 class="mb-2 text-lg font-medium">macOS Installation</h3>
					<ol class="list-decimal space-y-1 pl-5 text-gray-600">
						<li>Download the macOS disk image (.dmg) file</li>
						<li>Open the disk image and drag Eurora to the Applications folder</li>
						<li>Open Eurora from the Applications folder</li>
						<li>
							If prompted about security settings, follow the instructions to allow
							the app
						</li>
					</ol>
				</div>
				<div>
					<h3 class="mb-2 text-lg font-medium">Linux Installation</h3>
					<ol class="list-decimal space-y-1 pl-5 text-gray-600">
						<li>
							Download the appropriate package for your distribution (.deb, .rpm, or
							.AppImage)
						</li>
						<li>For .deb: <code>sudo dpkg -i eurora.deb</code></li>
						<li>For .rpm: <code>sudo rpm -i eurora.rpm</code></li>
						<li>For .AppImage: Make the file executable and run it directly</li>
					</ol>
				</div>
			</div>
		</Card.Content>
	</Card.Root>
</div>
