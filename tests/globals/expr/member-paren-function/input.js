;(function() {
  const bar = 42;
  const foo = bar;
  const win = window;
  const qux = (foo || bar);
}).call(this);
