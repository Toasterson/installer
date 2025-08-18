# FrontendNg

This project was generated using [Angular CLI](https://github.com/angular/angular-cli) version 20.1.6.

## Development server

To start a local development server, run:

```bash
ng serve
```

Once the server is running, open your browser and navigate to `http://localhost:4200/`. The application will automatically reload whenever you modify any of the source files.

## Code scaffolding

Angular CLI includes powerful code scaffolding tools. To generate a new component, run:

```bash
ng generate component component-name
```

For a complete list of available schematics (such as `components`, `directives`, or `pipes`), run:

```bash
ng generate --help
```

## Building

To build the project run:

```bash
ng build
```

This will compile your project and store the build artifacts in the `dist/` directory. By default, the production build optimizes your application for performance and speed.

## Running unit tests

To execute unit tests with the [Karma](https://karma-runner.github.io) test runner, use the following command:

```bash
ng test
```

## Running end-to-end tests

For end-to-end (e2e) testing, run:

```bash
ng e2e
```

Angular CLI does not come with an end-to-end testing framework by default. You can choose one that suits your needs.

## Additional Resources

For more information on using the Angular CLI, including detailed command references, visit the [Angular CLI Overview and Command Reference](https://angular.dev/tools/cli) page.


## API type generation (OpenAPI)

The local server exposes an OpenAPI spec at:

- http://localhost:8080/api/openapi.json

This project generates TypeScript types (no runtime client) from the spec using `openapi-typescript`, following Angular best practices to keep API models strongly typed while remaining framework-agnostic.

Scripts added in package.json:

- `yarn gen:api` — generates types into `src/api/openapi-types.ts` from the running server.
- `prebuild` and `prestart` — automatically attempt to regenerate the types before `build` and `start`. If the server is not running, the build/start will continue using the last generated types.

Manual generation:

```bash
yarn gen:api
```

Output file:

- `src/api/openapi-types.ts`

Example usage in Angular code (types only):

```ts
// Import types where needed
import type { paths } from '../api/openapi-types';

// Example: derive a request body type for POST /install
export type InstallRequestBody =
  paths['/install']['post']['requestBody']['content']['application/json'];
```

Notes:
- Ensure your API server is running locally on port 8080 for fresh generation.
- The generation is types-only to keep components and services lean and fully typed.
