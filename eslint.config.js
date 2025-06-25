import { fileURLToPath } from "node:url";

import { includeIgnoreFile } from "@eslint/compat";
import eslint from "@eslint/js";
import { defineConfig } from "eslint/config";
import simpleImportSort from "eslint-plugin-simple-import-sort";
import unicorn from "eslint-plugin-unicorn";
import tseslint from "typescript-eslint";

const ignorePath = fileURLToPath(new URL(".prettierignore", import.meta.url));

export default defineConfig(
  includeIgnoreFile(ignorePath),
  eslint.configs.recommended,
  // @ts-expect-error -- Weird bug in typescript-eslint
  tseslint.configs.strictTypeChecked,
  tseslint.configs.stylisticTypeChecked,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  unicorn.configs.recommended,
  {
    plugins: {
      "simple-import-sort": simpleImportSort,
    },
    rules: {
      "simple-import-sort/imports": "error",
      "simple-import-sort/exports": "error",
    },
  },
  {
    rules: {
      //
      // typescript-eslint
      //
      "@typescript-eslint/array-type": ["warn", { default: "array-simple" }],
      "@typescript-eslint/consistent-type-definitions": ["warn", "type"],
      "@typescript-eslint/explicit-function-return-type": [
        "warn",
        { allowExpressions: true },
      ],
      "@typescript-eslint/no-import-type-side-effects": "warn",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_", args: "all" },
      ],
      "@typescript-eslint/no-unnecessary-condition": [
        "warn",
        { allowConstantLoopConditions: true },
      ],
      "@typescript-eslint/promise-function-async": "warn",
      "@typescript-eslint/strict-boolean-expressions": [
        "warn",
        {
          allowString: true,
          allowNullableString: true,
          allowNullableBoolean: true,
          allowAny: true,
        },
      ],
      "@typescript-eslint/switch-exhaustiveness-check": "warn",
      "@typescript-eslint/restrict-template-expressions": [
        "warn",
        { allowBoolean: true, allowNumber: true, allowNullish: true },
      ],
      "@typescript-eslint/return-await": ["warn", "always"],

      //
      // vanilla eslint
      //
      "no-console": "warn",
      "no-duplicate-imports": "warn",
      // var is useful in `try-catch` blocks.
      "no-var": "off",
      "no-warning-comments": ["warn", { terms: ["xxx"] }],
      "object-shorthand": "warn",
      "prefer-const": ["warn", { destructuring: "all" }],
      yoda: "warn",

      //
      // unicorn
      //
      // Useless until https://github.com/sindresorhus/eslint-plugin-unicorn/issues/1993 is fixed.
      "unicorn/custom-error-definition": "off",
      // Too many false positives, doesn't know when using non-Array.
      // See https://github.com/sindresorhus/eslint-plugin-unicorn/issues/1394.
      "unicorn/no-array-method-this-argument": "off",
      // As above.
      "unicorn/no-array-callback-reference": "off",
      // How is this more readable?
      "unicorn/no-await-expression-member": "off",
      // Weird rule.
      "unicorn/no-keyword-prefix": "off",
      // Allow both null and undefined.
      "unicorn/no-null": "off",
      // Ridiculous.
      "unicorn/prevent-abbreviations": "off",
      // We're only building for browsers. window is fine.
      "unicorn/prefer-global-this": "off",
      // Typescript no likey.
      "unicorn/prefer-json-parse-buffer": "off",
      // Much easier to copy-paste/grep full attribute names.
      "unicorn/prefer-dom-node-dataset": "off",
      // Too many false positives: https://github.com/sindresorhus/eslint-plugin-unicorn/issues/2149
      "unicorn/prefer-top-level-await": "off",
      "unicorn/template-indent": [
        "warn",
        {
          indent: 2,
          // Forces proper indentation for *all* template literals.
          // If you need to have more control over spacing, use something like [...].join('\n') instead.
          tags: [],
          functions: [],
          selectors: ["TemplateLiteral"],
        },
      ],
    },
  },
);
