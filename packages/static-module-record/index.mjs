import {spawn} from 'child_process';
import {readFileSync} from 'fs';

const {freeze, keys} = Object;

const command = "basalt";
const args = ["transform", "-j", "-"];

/*
 *  Analyze a module calling out to basalt(1) writing the source
 *  to stdin and parsing the response JSON payload from stdout.
 */
function analyzeModule({source}) {
  return new Promise((resolve, reject) => {
    let child = spawn(command, args);
    let buf = Buffer.from([]);
    child.stdout.on('data', function (chunk) {
      buf = Buffer.concat([buf, chunk]);
    });
    child.stdin.setEncoding('utf-8');
    child.stdin.write(source);
    child.stdin.end();

    child.on('error', (e) => reject(e));

    child.on('close', function(code) {
      if (code === 0) {
        return resolve(JSON.parse(buf.toString()));
      }
      reject(new Error("Static module record program basalt(1) did not exit successfully"));
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

// node packages/static-module-record/index.mjs
//let source = readFileSync("tests/fixtures/static-module-record/main.js");
//let source = "export const avery = 'Avery'";
//let record = await parseModule(source);
//console.log(record);
