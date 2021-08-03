const doc = this.document;
console.log('foo');
fetch().then();

// Computed member property will only evaluate until the computed part
// and the globalThis should be stripped so this evaluates to `window`
const addEventListener = globalThis.window['addEventListener'];

// TODO: member expression in computed evaluation!
//const addEventListener = globalThis.document[ process.env.FOO ];

const isSimpleWindowsTerm = process.platform === 'win32' && !(process.env.TERM || '').toLowerCase().startsWith('xterm');

(versionA + '.').indexOf(versionB + '.');

[].slice.call(arguments);
