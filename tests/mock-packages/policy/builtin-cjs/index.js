const fs = require('fs');
const path = require('path');

function builtin() {
  fs.readSync("test.txt");
  path.join("a", "b", "c");
}
