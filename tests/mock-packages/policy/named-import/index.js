import {readSync} from 'fs';
import {join} from 'path';

function builtin() {
  readSync("test.txt");
  join("a", "b", "c");
}
