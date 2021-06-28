(({ imports: $h‍_imports , liveVar: $h‍_live , onceVar: $h‍_once  })=>{
    let bar, baz;
    $h‍_imports(new Map([
        [
            "./import-all-from-me.js",
            new Map([
                [
                    "*",
                    [
                        ($h‍_a)=>(bar = $h‍_a)
                        ,
                        ($h‍_a)=>(baz = $h‍_a)
                    ]
                ]
            ])
        ]
    ]), []);
});
