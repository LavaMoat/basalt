import test from 'ava'

import { run } from '../index'

test('call run from native code', (t) => {
  t.is(run(['basalt', '--version']), undefined)
})
