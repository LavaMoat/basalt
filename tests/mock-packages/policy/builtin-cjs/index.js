const fs = require('fs');
const path = require('path');
const Buffer = require('buffer').Buffer;

function builtin() {
  if (!Buffer.isBuffer(a)) return undefined;
  fs.readSync("test.txt");
  path.join("a", "b", "c");
}
