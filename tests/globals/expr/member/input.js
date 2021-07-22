const doc = this.document;
console.log('foo');
fetch().then();
// Computed member property will only evaluate until the computed part
// and the globalThis should be stripped so this evaluates to `window`
const addEventListener = globalThis.window['addEventListener'];

const isSimpleWindowsTerm = process.platform === 'win32' && !(process.env.TERM || '').toLowerCase().startsWith('xterm');

[].slice.call(arguments);

//(versionA + '.').indexOf(versionB + '.')
