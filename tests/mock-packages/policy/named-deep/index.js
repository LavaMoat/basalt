import process, {env} from 'process';

// Use the local symbol
env.DEEP1 = 'foo';
// Use the default import
process.env.DEEP2 = 'foo';
