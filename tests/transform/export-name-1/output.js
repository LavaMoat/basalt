(({ imports: $h‍_imports , liveVar: $h‍_live , onceVar: $h‍_once  })=>{
    $h‍_imports(new Map([]), []);
    let abc = 123;
    $h‍_once.abc(abc);
    let $c‍_def = 456;
    $h‍_live.def($c‍_def);
    let def2 = def;
    $h‍_once.def2(def2);
    def++;
    const ghi = 789;
    $h‍_once.ghi(ghi);
});
