import fs from 'fs';
import path from 'path';
import {readSync} from 'fs';
import * as fs from 'fs';

const file = readSync("test.txt");
const other = fs.readSync("test.txt");

const pth = path.join("a", "b", "c");
