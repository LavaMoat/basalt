import fs from 'fs';
import path from 'path';

function builtin() {
  fs.readSync("test.txt");
  path.join("a", "b", "c");
}
