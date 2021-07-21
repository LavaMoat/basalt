const doc = this.document;
console.log('foo');
fetch().then();
// Computed member property will only evaluate until the computed part
const addEventListener = globalThis.window['addEventListener'];

const isSimpleWindowsTerm = process.platform === 'win32' && !(process.env.TERM || '').toLowerCase().startsWith('xterm');

// Works
//!(process.env.TERM || '');
// Fails
//!(process.env.TERM || '').toLowerCase();
