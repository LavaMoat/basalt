import {env} from 'process';

// Use the local symbol
env.DEEP1 = 'foo';
// Use the globally exposed builtin module
process.env.DEEP2 = 'foo';
