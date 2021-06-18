(({ imports: $h‍_imports , liveVar: $h‍_live , onceVar: $h‍_once  })=>{
    $h‍_imports(new Map([]), []);
    const fn2 = fn;
    $h‍_once.fn2(fn2);
    Object.defineProperty($c‍_fn, "name", {
        value: "fn"
    });
    $h‍_live.fn($c‍_fn);
    function $c‍_fn() {
        return 'foo';
    };
    const fn3 = fn;
    $h‍_once.fn3(fn3);
});
