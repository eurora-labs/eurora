<script lang="ts">
	import { Card, Button } from '@eurora/ui';
	import { Users, Briefcase, GraduationCap, Mail, Github, Linkedin } from 'lucide-svelte';
    // Use static images instead of importing from lib
    const laura_thommen_img = '/images/avatars/laura_thommen.jpg';
    const andre_roelofs_img = '/images/avatars/andre_roelofs.jpg';
    const linda_rosner_img = '/images/avatars/linda_rosner.jpg';
</script>

<div class="container mx-auto px-4 py-16 max-w-5xl">
	<div class="text-center mb-12">
		<h1 class="text-4xl font-bold mb-4">About Us</h1>
		<p class="text-xl text-gray-600 max-w-2xl mx-auto">
			Meet the team behind Eurora. We're passionate about creating intelligent AI solutions that
			respect your privacy.
		</p>
	</div>

	<!-- Company Overview -->
	<Card.Root class="p-6 mb-12">
		<Card.Header>
			<div class="flex items-center gap-2">
				<Users class="w-6 h-6 text-purple-600" />
				<Card.Title>Our Mission</Card.Title>
			</div>
		</Card.Header>
		<Card.Content>
			<p class="text-gray-600 mb-4">
				At Eurora, we believe in intelligence without compromise. Our mission is to create AI tools
				that enhance human capabilities while maintaining the highest standards of privacy and
				security. We're committed to developing technology that works for people, not the other way
				around.
			</p>
			<p class="text-gray-600">
				Founded in 2025, our team brings together expertise in artificial intelligence, privacy
				engineering, and user experience design to build the next generation of AI assistants that
				you can truly trust.
			</p>
		</Card.Content>
	</Card.Root>

	<!-- Leadership Team -->
	<h2 class="text-3xl font-bold mb-8">Leadership Team</h2>

	<div class="grid grid-cols-1 md:grid-cols-3 gap-8 mb-16">
		<!-- CEO -->
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex flex-col items-center mb-4">
					<img src={laura_thommen_img} alt="Laura Thommen" class="w-32 h-32 rounded-full object-cover mb-4" />

					<!-- <div class="w-32 h-32 rounded-full bg-gray-200 mb-4"></div> -->
					<Card.Title>Laura Thommen</Card.Title>
					<p class="text-purple-600 font-medium">Chief Executive Officer</p>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<Briefcase class="w-4 h-4 text-purple-600" /> Experience
						</h3>
						<p class="text-gray-600">
							Demonstrated breadth of experience from sales to management, driving new product adoption and revenue growth with over 30 years of experience working for manufacturers such as Apple, Adobe, Avid, DDN Storage and Northrop Grumman. 
						</p>
					</div>
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<GraduationCap class="w-4 h-4 text-purple-600" /> Education
						</h3>
						<p class="text-gray-600">MBA from Pepperdine University, BS in Business Administration with a minor in Finance from the University of Southern California.</p>
					</div>
					<div class="flex justify-center gap-3 mt-4">
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Mail class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Linkedin class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Github class="h-4 w-4" />
						</Button>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- CTO -->
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex flex-col items-center mb-4">
					<img src={andre_roelofs_img} alt="Andre Roelofs" class="w-32 h-32 rounded-full object-cover mb-4" />
					<Card.Title>Andre Roelofs</Card.Title>
					<p class="text-purple-600 font-medium">Chief Technology Officer</p>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<Briefcase class="w-4 h-4 text-purple-600" /> Experience
						</h3>
						<p class="text-gray-600">
							Founder and CTO of Eurora Labs, previously Head of Engineering and Machine Learning at Cuebric. Led development of award-winning AI solutions, specializing in software architecture, machine learning, and computer vision.
						</p>
					</div>
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<GraduationCap class="w-4 h-4 text-purple-600" /> Education
						</h3>
						<p class="text-gray-600">
							BS in Computer Science from Avans University of Applied Sciences; graduate studies in AI at Maastricht University.
						</p>
					</div>
					<div class="flex justify-center gap-3 mt-4">
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Mail class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Linkedin class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Github class="h-4 w-4" />
						</Button>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- COO -->
		<Card.Root class="p-6">
			<Card.Header>
				<div class="flex flex-col items-center mb-4">
					<img src={linda_rosner_img} alt="Laura Thommen" class="w-32 h-32 rounded-full object-cover mb-4" />

					<Card.Title>Linda Rosner</Card.Title>
					<p class="text-purple-600 font-medium">Chief Operations Officer</p>
				</div>
			</Card.Header>
			<Card.Content>
				<div class="space-y-4">
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<Briefcase class="w-4 h-4 text-purple-600" /> Experience
						</h3>
						<p class="text-gray-600">
							Co-founder of ArtisansPR, with deep expertise in media, entertainment, and tech communications. Serves on HPA’s Awards Committee and formerly chaired SMPTE Hollywood’s Board. Founding Board Member of EIPMA and frequent panel moderator for NAB, SMPTE, and others.
						</p>
					</div>
					<div>
						<h3 class="font-medium flex items-center gap-2">
							<GraduationCap class="w-4 h-4 text-purple-600" /> Education
						</h3>
						<p class="text-gray-600">
							BA in Theater from UCLA.
						</p>
					</div>
					<div class="flex justify-center gap-3 mt-4">
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Mail class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Linkedin class="h-4 w-4" />
						</Button>
						<Button variant="ghost" size="icon" class="h-8 w-8">
							<Github class="h-4 w-4" />
						</Button>
					</div>
				</div>
			</Card.Content>
		</Card.Root>
	</div>

	<!-- Space for More Team Members -->
	<Card.Root class="p-6 mb-16 bg-purple-50 border-none">
		<Card.Content class="text-center">
			<h2 class="text-2xl font-bold mb-4">Join Our Team</h2>
			<p class="text-gray-600 max-w-2xl mx-auto mb-6">
				We're always looking for talented individuals who are passionate about AI, privacy, and
				creating exceptional user experiences. Check out our open positions and become part of our
				journey.
			</p>
			<Button href="/contact" variant="default">View Open Positions</Button>
		</Card.Content>
	</Card.Root>

	<!-- Company Values -->
	<h2 class="text-3xl font-bold mb-8">Our Values</h2>
	<div class="grid grid-cols-1 md:grid-cols-2 gap-8 mb-16">
		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Privacy First</Card.Title>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600">
					We believe that privacy is a fundamental right. All our products are designed with privacy
					as a core principle, not an afterthought.
				</p>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>User-Centered Design</Card.Title>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600">
					We create technology that adapts to how people work, not the other way around. Our
					products are intuitive, accessible, and designed for real human needs.
				</p>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Ethical AI</Card.Title>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600">
					We're committed to developing AI that is fair, transparent, and beneficial to society. We
					actively work to identify and mitigate biases in our systems.
				</p>
			</Card.Content>
		</Card.Root>

		<Card.Root class="p-6">
			<Card.Header>
				<Card.Title>Continuous Innovation</Card.Title>
			</Card.Header>
			<Card.Content>
				<p class="text-gray-600">
					We're never satisfied with the status quo. We continuously push the boundaries of what's
					possible to deliver ever-improving experiences to our users.
				</p>
			</Card.Content>
		</Card.Root>
	</div>

	<!-- Call to Action -->
	<Card.Root class="p-8 bg-purple-50 border-none">
		<Card.Content class="text-center">
			<h2 class="text-3xl font-bold mb-4">Want to Learn More?</h2>
			<p class="text-xl text-gray-600 max-w-2xl mx-auto mb-6">
				Have questions about our team or company? We'd love to hear from you.
			</p>
			<Button href="/contact" size="lg" class="px-8">Contact Us</Button>
		</Card.Content>
	</Card.Root>
</div>
