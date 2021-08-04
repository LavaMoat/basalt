class Foo {
  // This does NOT shadow as it requires an explicit `this` reference.
  fetch = async () => {}

  foo() {
    return fetch('/api/foo');
  }
}
