/*
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

function atob(str) {
  return Buffer.from(str, 'base64').toString('binary');
}

var util;
util = require('util');

const Foo = function(){}
util.inherits(Foo, EE);

module.exports = require('url');

var os = require('os');
new Buffer(os.EOL);

*/

var isWin32 = require('os').platform() === 'win32';

/*
function foo() {
  var Buffer = require('buffer').Buffer
  return Buffer.isBuffer([])
}

function bar(arr) {
  var body = Buffer.isBuffer(arr[0]) ? Buffer.concat(arr)
    : arr.join('')
}
*/

/*
function bar() {
  var map = this.toString();
  return new Buffer(map).toString('base64');
};
*/

/*
function foo() {
    return {
      visitor: {
        CallExpression(path, state) {
          console.warn('blah');
        }
      }
    }
}
*/
