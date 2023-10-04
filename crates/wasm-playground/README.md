## Comparison with hand-written WAT:

```
wasm_fold_add_square    time:   [283.08 µs 284.57 µs 286.20 µs]
                        change: [-2.8707% -2.2522% -1.6547%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 1 outliers among 100 measurements (1.00%)
  1 (1.00%) high mild

interp_fold_add_square  time:   [15.614 ms 15.644 ms 15.674 ms]
                        change: [-1.6398% -1.3131% -1.0018%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 4 outliers among 100 measurements (4.00%)
```

## ExternRef (latest)
```
fold-add-square         time:   [2.2729 ms 2.2791 ms 2.2853 ms]
                        change: [-12.184% -10.084% -8.4434%] (p = 0.00 < 0.05)
                        Performance has improved.
```