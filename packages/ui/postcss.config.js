import autoprefixer from 'autoprefixer';
import postcssImport from 'postcss-import';

export default {
	plugins: [postcssImport, autoprefixer]
};
// import defaultConfig from '@eurora/ui/postcss.config';
// import tailwindcss from 'tailwindcss';

// export default {
// 	plugins: [tailwindcss('./tailwind.config'), ...defaultConfig.plugins]
// };
