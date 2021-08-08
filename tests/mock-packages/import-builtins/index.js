import fs from 'fs';
import path from 'path';
import {readSync, writeSync} from 'fs';
import * as fs from 'fs';

const file = readSync("test.txt");
const other = fs.readSync("test.txt");
fs.writeSync("test.txt", "foo");

const pth = path.join("a", "b", "c");

class Foo {
  addComment() {
    return super.addComment();
  }
}
