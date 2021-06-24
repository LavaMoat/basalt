# Basalt

LavaMoat analyzer, linter and bundler.

To get started [install Rust][rust-install] then add the `nightly` toolchain:

```
rustup toolchain install nightly
```

Now you can compile and test using `cargo`.

## Test

To run the tests:

```
cargo +nightly test -- --nocapture
```

## List

To print the module graph for a file:

```
cargo +nightly run -- ls tests/fixtures/basic-tree/main.js
```

## Static Module Record

To print the static module record meta data for a file:

```
cargo +nightly run -- meta tests/fixtures/static-module-record/main.js
```

To print the static module record functor for a file:

```
cargo +nightly run -- transform tests/fixtures/static-module-record/main.js
```

## API Documentation

```
cargo +nightly doc --open --no-deps
```

[rust-install]: https://www.rust-lang.org/tools/install

## Release Build

```
cargo +nightly build --release
```

Then copy the executable into `PATH`, for example:

```
cp -r target/release/basalt ~/bin
```
