# Contributing to rtrace

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Node.js](https://nodejs.org/) 16 or later
- npm

Install JavaScript dependencies before using the Node.js binding or task runner:

```bash
npm install
```

## Local development loop

Build the complete workspace:

```bash
cargo build --workspace
npm run build:all
```

Run the Rust and Node.js test suites:

```bash
cargo test --workspace
npm test
npm run test:all
```

Check formatting and linting:

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
```

The [task runner guide](TASKS.md) documents shortcuts for building, testing,
rendering examples, schema validation, formatting, and CI checks. For example:

```bash
npx hereby dev
npx hereby precommit
npx hereby ci
```

Regenerate documentation images after rendering changes:

```bash
npx hereby doc:render
```

## Working on documentation and examples

Feature and scene-format documentation lives in [`doc/README.md`](doc/README.md).
Example scenes and their rendered outputs live in [`examples/`](examples/).
Keep generated images unchanged unless the rendering code or scene has changed.

## Pull requests

1. Create a focused feature branch.
2. Make the change and add or update tests.
3. Run the local development checks above.
4. Update relevant documentation.
5. Open a pull request describing the behavior change and validation performed.

## Troubleshooting

For build errors, update the stable Rust toolchain and try `cargo clean`.
For Node.js binding issues, verify the Node.js version and rebuild with
`npm run build:node`.

## Publishing

Publishing is intentionally separate from the local development loop:

```bash
cargo publish -p rtrace
cargo publish -p rtrace-cli
```

The npm package is published by [`.github/workflows/npm-publish.yml`](.github/workflows/npm-publish.yml).
It builds platform-qualified native addons on Linux, macOS, and Windows, assembles
them and matching `rtrace` CLI executables into `dist/`, and then runs `npm
publish` using npm Trusted Publishing (GitHub Actions OIDC). Configure the npm
package's trusted publisher to this repository, workflow (`npm-publish.yml`),
and environment (`npm-publish`) before publishing. Start it manually from
GitHub or push a `v*` tag. A local `npm publish` only contains artifacts built
on the current machine.
