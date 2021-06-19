(({ imports: $h‍_imports , liveVar: $h‍_live , onceVar: $h‍_once  })=>{
    $h‍_imports(new Map([]), []);
    const abc = 123;
    $h‍_once.abc(abc);
    const { def , nest: [, ghi, ...nestrest] , ...rest } = {
        def: 456,
        nest: [
            'skip',
            789,
            'a',
            'b'
        ],
        other: 999,
        and: 998
    };
    $h‍_once.def(def);
    $h‍_once.ghi(ghi);
    $h‍_once.nestrest(nestrest);
    $h‍_once.rest(rest);
});
