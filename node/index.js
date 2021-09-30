const { loadBinding } = require('@node-rs/helper')

const fs = require('fs')
const path = require('path')


const files = fs.readdirSync(path.join(__dirname));
console.log(files);


/**
 * __dirname means load native addon from current dir
 * 'package-template' means native addon name is `package-template`
 * the first arguments was decided by `napi.name` field in `package.json`
 * the second arguments was decided by `name` field in `package.json`
 * loadBinding helper will load `package-template.[PLATFORM].node` from `__dirname` first
 * If failed to load addon, it will fallback to load from `@napi-rs/package-template-[PLATFORM]`
 */
module.exports = loadBinding(__dirname, 'basalt', '@lavamoat/basalt')
