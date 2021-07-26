const {readSync} = require('fs');
const {join} = require('path');

function builtin() {
  readSync("test.txt");
  join("a", "b", "c");
}
