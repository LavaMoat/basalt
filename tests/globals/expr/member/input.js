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

const o = () => {};
function Bn(e) {
  var t = !e;
  void 0 ===
    (e = o(
      e,
      {
        ascii_only: !1,
        beautify: !1,
        braces: !1,
        comments: "some",
        ecma: 5,
        ie8: !1,
        indent_level: 4,
        indent_start: 0,
        inline_script: !0,
        keep_numbers: !1,
        keep_quoted_props: !1,
        max_line_len: !1,
        preamble: null,
        preserve_annotations: !1,
        quote_keys: !1,
        quote_style: 0,
        safari10: !1,
        semicolons: !0,
        shebang: !0,
        shorthand: void 0,
        source_map: null,
        webkit: !1,
        width: 80,
        wrap_iife: !1,
        wrap_func_args: !0,
      },
      !0
    )).shorthand;
}

function atob(str) {
  return Buffer.from(str, 'base64').toString('binary');
}
