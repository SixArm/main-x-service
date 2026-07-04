import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';

export default ts.config(
    {
        ignores: [
            '.svelte-kit/',
            'build/',
            'dist/',
            'node_modules/',
            'playwright-report/',
            'test-results/',
            'src/lib/api/schema.d.ts'
        ]
    },
    js.configs.recommended,
    ...ts.configs.recommended,
    ...svelte.configs.recommended,
    {
        languageOptions: {
            globals: { ...globals.browser, ...globals.node }
        }
    },
    {
        files: ['**/*.svelte', '**/*.svelte.ts'],
        languageOptions: {
            parserOptions: { parser: ts.parser }
        }
    },
    {
        rules: {
            // Underscore prefix marks intentionally-unused bindings (test
            // stubs discard rest props / no-op arguments).
            '@typescript-eslint/no-unused-vars': [
                'error',
                {
                    argsIgnorePattern: '^_',
                    varsIgnorePattern: '^_',
                    caughtErrorsIgnorePattern: '^_',
                    ignoreRestSiblings: true
                }
            ],
            // This app uses plain same-origin hrefs / goto throughout and
            // has not adopted SvelteKit's resolve() typed-route helper.
            'svelte/no-navigation-without-resolve': 'off',
            // Conflicts with form fields that are writable but seeded from
            // `data` props via $effect.pre (e.g. the rename/search inputs).
            'svelte/prefer-writable-derived': 'off'
        }
    }
);
