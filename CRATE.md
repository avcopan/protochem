# `protochem`

Rust learning sandbox for implementing cheminformatics algorithms

This is the Rust library behind the [`protochem`](https://pypi.org/project/protochem/) Python package.
It has no Python dependency.

```rust
let carbon = protochem::Element::from_symbol("C").expect("carbon is in the table");
assert_eq!(carbon.atomic_number, 6);
```

See the [documentation site](https://avcopan.github.io/protochem/) for the Python API, and the [repository](https://github.com/avcopan/protochem) for the source code.
