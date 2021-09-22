#!/usr/bin/env node

const { run } = require('./index.js')

// Remove the script name otherwise argument parsing will fail
process.argv.splice(1, 1)
// The USAGE help parses the program name from the binary name
// which displays `node` incorrectly
process.argv[0] = 'basalt'
run(process.argv)
