const kind = typeof document;
//const isSimpleWindowsTerm = process.platform === 'win32' && !(process.env.TERM || '').toLowerCase().startsWith('xterm');

// Works
!(process.env.TERM || '');
// Fails
!(process.env.TERM || '').toLowerCase();
