const fs = require('fs');
const path = require('path');
const {readSync, writeSync} = require('fs');
const EE = require('events').EventEmitter;

const file = readSync("test.txt");
const other = fs.readSync("test.txt");
writeSync("test.txt", "foo");

const pth = path.join("a", "b", "c");
const rs = readSync;



// Write to builtin(s)!
path.join.a.b.c = function() {}
fs = {}
writeSync++;

function Foo() {
  EE.call(this);
}

var crypto = require('crypto')

function random () {
  var buf = crypto
    .randomBytes(4)
    .toString('hex')
  return parseInt(buf, 16);
}
