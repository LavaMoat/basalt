const o = new Object();
Object.defineProperty(/**/);
const f = Function();
const F = new Function();
const b = Boolean('foo');
const s = Symbol('foo');
const e = new Error('Some error');
const errors = [
  Error,
  AggregateError,
  EvalError,
  InternalError,
  RangeError,
  ReferenceError,
  SyntaxError,
  TypeError,
  URIError,
];

const num = Number(10);
const bigInt = BigInt('9007199254740991');
const min = Math.min(1, 0);
const now = Date.now();
const msg = new String('Some value');
const re = new RegExp('foo', 'gi');

const a = Array.from([1,2,3]);
const A = new Array(0);

function foo() {
  const args = arguments;
  const callee = arguments.callee;
}
