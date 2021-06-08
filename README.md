# Basalt

LavaMoat analyzer, linter and bundler.

Currently the `swc` project should be checked out in the parent directory as we are building from source; later we will build against the packages in the cargo repository.

## List

To print the module graph for a file:

```
cargo +nightly run -- ls test/fixtures/basic-tree/main.js
```

## Static Module Record

To print the static module record for a file:

```
cargo +nightly run -- smr test/fixtures/static-module-record/main.js
```

## API Documentation

```
cargo +nightly doc --open --no-deps
```
