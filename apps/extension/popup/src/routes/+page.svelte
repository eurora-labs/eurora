<script lang="ts">
	import { EurIcons, Button, Separator } from '@eurora/ui';
	import { SiGithub } from '@icons-pack/svelte-simple-icons';
	import { Globe } from 'lucide-svelte';
</script>

<div class="flex flex-col items-center min-w-[600px] justify-between min-h-[300px] p-6 bg-background text-foreground rounded-lg">
    <!-- Header with Logo -->
    <div class="flex flex-col items-center mt-6">
        <div class="relative">
            <div class="absolute -inset-1 bg-gradient-to-r from-primary/20 to-secondary/20 rounded-full blur-md"></div>
            <EurIcons.EuroraLogo
                class="w-28 h-28 mb-2 relative"
            />
        </div>
        <h1 class="text-3xl font-bold mt-3 bg-gradient-to-r from-primary to-secondary bg-clip-text text-transparent">Eurora Labs</h1>
    </div>
    
    <!-- Content Area (can be used for additional content) -->
    <div class="flex-grow w-full flex items-center justify-center">
        <!-- You can add additional content here if needed -->
    </div>

    <Separator />
    
    <!-- Footer with Icons -->
    <div class="flex items-center justify-center gap-6 mt-6 w-full pt-4">
        <Button variant="ghost" size="icon" href="https://eurora-labs.com" class="hover:bg-muted rounded-full transition-colors">
            <div class="w-5 h-5 text-muted-foreground hover:text-foreground">
                <Globe size={20} />
            </div>
        </Button>
        <Button variant="ghost" size="icon" href="https://github.com/Eurora-Labs/eurora" class="hover:bg-muted rounded-full transition-colors">
            <div class="w-5 h-5 text-muted-foreground hover:text-foreground">
                <SiGithub size={20} />
            </div>
        </Button>
    </div>
</div>