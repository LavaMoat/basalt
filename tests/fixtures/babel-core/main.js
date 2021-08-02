//require('@babel/core');
//require('@babel/code-frame');
//require('ansi-styles');
//require('chalk');

//require('http');

// Ok
//import '@babel/helper-validator-identifier';
//import '@babel/helper-optimise-call-expression';
//import '@babel/helper-member-expression-to-functions';
//import '@babel/helper-split-export-declaration';
//import '@babel/helper-simple-access';
//import '@babel/helper-module-imports';
//import '@babel/types';
//import '@babel/template';
//import '@babel/traverse';
//import '@babel/helper-module-transforms';
//import '@babel/helper-replace-supers';

//
//import browserslist from 'browserslist';
//import bufferEqual from 'buffer-equal';

//import '@babel/plugin-transform-modules-umd';

//import '@choojs/findup';

// Infinite loop
//import '@lavamoat/lavapack';

//import '@babel/helper-validator-identifier';
//
//import '@babel/types';

//import '@babel/parser';
//import '@babel/traverse';

//import 'yazl';
//
//import 'yaml';
//
class Foo {
  constructor(firstItem) {
    super(firstItem.type === PlainValue.Type.SEQ_ITEM ? PlainValue.Type.SEQ : PlainValue.Type.MAP);

    for (let i = firstItem.props.length - 1; i >= 0; --i) {
      if (firstItem.props[i].start < firstItem.context.lineStart) {
        // props on previous line are assumed by the collection
        this.props = firstItem.props.slice(0, i + 1);
        firstItem.props = firstItem.props.slice(i + 1);
        const itemRange = firstItem.props[0] || firstItem.valueRange;
        firstItem.range.start = itemRange.start;
        break;
      }
    }

    this.items = [firstItem];
    const ec = grabCollectionEndComments(firstItem);
    if (ec) Array.prototype.push.apply(this.items, ec);
  }
}
