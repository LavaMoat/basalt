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

;(function() {
    baseForOwn(LazyWrapper.prototype, function(func, methodName) {
      var checkIteratee = /^(?:filter|find|map|reject)|While$/.test(methodName),
          isTaker = /^(?:head|last)$/.test(methodName),
          lodashFunc = lodash[isTaker ? ('take' + (methodName == 'last' ? 'Right' : '')) : methodName],
          retUnwrapped = isTaker || /^find/.test(methodName);

      if (!lodashFunc) {
        return;
      }
      lodash.prototype[methodName] = function() {
        var value = this.__wrapped__,
            args = isTaker ? [1] : arguments,
            isLazy = value instanceof LazyWrapper,
            iteratee = args[0],
            useLazy = isLazy || isArray(value);

        var interceptor = function(value) {
          var result = lodashFunc.apply(lodash, arrayPush([value], args));
          return (isTaker && chainAll) ? result[0] : result;
        };

        if (useLazy && checkIteratee && typeof iteratee == 'function' && iteratee.length != 1) {
          // Avoid lazy use if the iteratee has a "length" value other than `1`.
          isLazy = useLazy = false;
        }
        var chainAll = this.__chain__,
            isHybrid = !!this.__actions__.length,
            isUnwrapped = retUnwrapped && !chainAll,
            onlyLazy = isLazy && !isHybrid;

        if (!retUnwrapped && useLazy) {
          value = onlyLazy ? value : new LazyWrapper(this);
          var result = func.apply(value, args);
          result.__actions__.push({ 'func': thru, 'args': [interceptor], 'thisArg': undefined });
          return new LodashWrapper(result, chainAll);
        }
        if (isUnwrapped && onlyLazy) {
          return func.apply(this, args);
        }
        result = this.thru(interceptor);
        return isUnwrapped ? (isTaker ? result.value()[0] : result.value()) : result;
      };
    });

}).call(this);
