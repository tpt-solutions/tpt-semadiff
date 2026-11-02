# Engine benchmarks

Release build, one machine (Windows 11), one edit per file (the first `fn` is renamed).
Reproduce with:

```
cargo run --release -p tpt-semadiff-core --example bench -- <files...>
```

| file | lines | nodes | chawathe | zhang-shasha | ops |
| --- | ---: | ---: | ---: | ---: | ---: |
| gumtree.rs | 202 | 2350 | 6.5 ms | 1074.7 ms | 1 |
| ops.rs | 172 | 1748 | 4.8 ms | 686.9 ms | 1 |
| parse.rs | 174 | 1952 | 5.5 ms | 650.9 ms | 1 |
| report.rs | 253 | 2587 | 7.1 ms | 3120.2 ms | 1 |
| geometry.rs (accesskit 0.16.3) | 880 | 7346 | 43.5 ms | 22300.0 ms | 1 |

Zhang-Shasha grows roughly with the product of the two trees' sizes squared in the worst
case. On these files it is 100–500× slower than the default engine, which is why Chawathe
(GumTree-style) is the default. Zhang-Shasha remains available with `--engine zhang-shasha`
for small files where an exact minimum mapping matters.
