// @ts-check

import {spawn, execSync} from 'child_process';
import {readFileSync} from 'fs';

const {freeze, keys} = Object;

/*
 *  Analyze a module calling out to basalt(1) writing the source
 *  to stdin and parsing the response JSON payload from stdout.
 */
function analyzeModule({source}) {
  const command = "basalt";
  const args = ["transform", "-j", "-"];
  return new Promise((resolve, reject) => {
    let child = spawn(command, args);
    let buf = Buffer.from([]);
    child.stdout.on('data', function (chunk) {
      buf = Buffer.concat([buf, chunk]);
    });
    child.stdin.setEncoding('utf-8');
    child.stdin.write(source);
    child.stdin.end();
    child.once('error', (e) => reject(e));
    child.once('close', function(code) {
      if (code === 0) {
        try {
          resolve(JSON.parse(buf.toString()));
        } catch (e) {
          reject(e);
        }
      } else {
        reject(
            new Error(
              "Static module record program basalt(1) did not exit successfully"));
      }
    })
  })
}

export async function parseModule(source, url) {
  const {
    meta,
    program,
  } = await analyzeModule({ source, url });

  return freeze({
    imports: freeze([...keys(meta.imports)].sort()),
    exports: freeze(
      [...keys(meta.liveExportMap), ...keys(meta.fixedExportMap)].sort(),
    ),
    reexports: freeze([...meta.exportAlls].sort()),
    __syncModuleProgram__: program,
    __liveExportMap__: meta.liveExportMap,
    __fixedExportMap__: meta.fixedExportMap,
  })
}

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

/** @type {import('./types.js').ParseFn} */
export const parseArchiveMjs = async (
  bytes,
  _specifier,
  _location,
  _packageLocation,
) => {
  const source = textDecoder.decode(bytes);
  const record = await parseModule(bytes);
  const pre = textEncoder.encode(JSON.stringify(record));
  return {
    parser: 'pre-mjs-json',
    bytes: pre,
    record,
  };
};
