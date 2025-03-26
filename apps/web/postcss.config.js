import defaultConfig from '@eurora/ui/postcss.config';
import tailwindcss from 'tailwindcss';

export default {
	plugins: [tailwindcss('./tailwind.config'), ...defaultConfig.plugins]
};
