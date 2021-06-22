(({ imports: $h‍_imports , liveVar: $h‍_live , onceVar: $h‍_once  })=>{
    $h‍_imports(new Map([]), []);
    $h‍_live.abc();
    const abc2 = abc;
    $h‍_once.abc2(abc2);
    let $c‍_abc = 123;
    abc = $c‍_abc;
    const abc3 = abc;
    $h‍_once.abc3(abc3);
});
