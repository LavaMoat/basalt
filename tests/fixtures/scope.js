import foo from './foo.js';

function baz() {
  const local_qux = document;
}

class Qux {}

{
  const block_foo, block_qux;
  {
    const nested_foo, nested_qux;
  }

  while(true) {
    const while_foo, while_qux;
  }

  do {
    const do_while_foo, do_while_qux;
  } while(false);
}

with (p) {
  const with_foo, with_qux;
}

const a, b, c;
//var d;
//let e, f, g;

//export const blah = document;
