/* eslint-env node */
module.exports = {
  root: true,
  env: {
    browser: true,
    es2021: true,
    node: true,
  },
  extends: [
    'eslint:recommended',
    'plugin:vue/vue3-recommended',
    'plugin:@typescript-eslint/recommended',
  ],
  parser: 'vue-eslint-parser',
  parserOptions: {
    parser: '@typescript-eslint/parser',
    ecmaVersion: 'latest',
    sourceType: 'module',
  },
  rules: {
    // Coding Conventions: "Use 2 spaces for indentation"
    'indent': ['error', 2],
    'vue/html-indent': ['error', 2],
    'vue/script-indent': ['error', 2, { 'baseIndent': 0 }],

    // Coding Conventions: "Use const by default"
    'prefer-const': 'error',

    // Coding Conventions: "Naming: camelCase for variables"
    // (TypeScript標準ルールでカバーされますが、明示的にキャメルケースを強制する場合の設定例)
    // '@typescript-eslint/naming-convention': [
    //   'error',
    //   { selector: 'variable', format: ['camelCase', 'UPPER_CASE'] },
    //   { selector: 'function', format: ['camelCase'] },
    //   { selector: 'class', format: ['PascalCase'] }
    // ],

    // Vue 3 <script setup> Compiler Macros (defineProps etc) warning回避
    'vue/multi-word-component-names': 'off', // 任意のコンポーネント名を使う場合
  },
}