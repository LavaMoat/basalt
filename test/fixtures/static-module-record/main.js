import foo from './import-default-export-from-me.js';
import * as bar from './import-all-from-me.js';
import { fizz, buzz } from './import-named-exports-from-me.js';
import { color as colour } from './import-named-export-and-rename.js';
import 'core-js';

export let quuux = null;

export { qux } from './import-and-reexport-name-from-me.js';
export * from './import-and-export-all.js';
export default 42;
export const quux = 'Hello, World!';

const aleph = 0;
export { aleph as alpha };
export { grey as gray } from './reexport-name-and-rename.js';

// Late binding of an exported variable.
quuux = 'Hello, World!';
