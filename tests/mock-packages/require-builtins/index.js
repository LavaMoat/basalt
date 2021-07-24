const fs = require('fs');
const path = require('path');
const {readSync, writeSync} = require('fs');

const file = readSync("test.txt");
const other = fs.readSync("test.txt");
fs.writeSync("test.txt", "foo");

const pth = path.join("a", "b", "c");

// Write to builtin(s)!
path.join.a.b.c = function() {}
fs = {}
