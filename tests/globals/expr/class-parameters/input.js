const Foo = class {
  constructor(win = window) {}
  doSomething(log = console.log) {}
  #doSomethingPrivate(info = console.info) {}
};
