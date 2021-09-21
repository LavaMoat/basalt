import test from 'ava'

import { cli } from '../index'

test('call cli from native code', (t) => {
  const fixture = "foo"
  t.is(cli(fixture), fixture + 100)
})
