import test from 'ava'

import { cli } from '../index'

test('call cli from native code', (t) => {
  t.is(cli(['--help']), undefined)
})
