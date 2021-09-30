# Basalt

LavaMoat analyzer, linter and bundler.

## Install

```
npm i -g @lavamoat/basalt
```

## Development

To get started [install Rust][rust-install], then you can compile and test using `cargo`.

## Test

To run the tests first install dependencies with `yarn install` then link the mock packages:

```
./install-test-packages.sh
```

Afterwards you can run the tests:

```
cargo test -- --nocapture
```

## List

To print the module graph for a file:

```
cargo run -- tree tests/fixtures/basic-tree/main.js
```

## Static Module Record

To print the static module record meta data for a file:

```
cargo run -- debug meta tests/fixtures/static-module-record/main.js
```

To print the static module record functor for a file:

```
cargo run -- debug transform tests/fixtures/static-module-record/main.js
```

Use the `--json` options for a JSON document containing both the `meta` data and functor `program`:

```
cargo run -- debug transform tests/fixtures/static-module-record/main.js -j
```

## Globals

To print the global variables for a module:

```
cargo run -- debug globals tests/fixtures/globals/main.js
```

To print the scope tree used to compute the globals use the `--debug` option:

```
cargo run -- debug globals tests/fixtures/globals/main.js -d
```

### Compartment Mapper

To test the static module record transform in the context of the [compartment-mapper][] create a release build and copy `target/release/basalt` into `PATH`.

Then copy [parse-archive-mjs.js](/parse-archive-mjs.js) to overwrite [parse-archive-mjs.js](https://github.com/endojs/endo/blob/master/packages/compartment-mapper/src/parse-archive-mjs.js) and run `yarn test` in the [compartment-mapper][] directory.

## API Documentation

```
cargo doc --open --no-deps
```

[rust-install]: https://www.rust-lang.org/tools/install

## Release Build

```
cargo build --release
```

## Publish

To publish a release set the new version:

```
yarn version
```

Then update `Cargo.toml` and `node/package.json` to match the new version.

Now you can push to publish to the npm registry:

```
git push
```

[compartment-mapper]: https://github.com/endojs/endo/tree/master/packages/compartment-mapper
