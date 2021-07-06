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

  for (;;) {
    const for_foo, for_qux;
  }

  for (let z in {a: 'a', b: 'b'}) {
    const for_in_foo, for_in_qux;
  }

  for (let i of [1,2,3]) {
    const for_of_foo, for_of_qux;
  }
}

with (p) {
  const with_foo, with_qux;
}

const a, b, c;
//var d;
//let e, f, g;

//export const blah = document;
