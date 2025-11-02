# cached-fn

`cached-fn` provides `CachedFn`, a lightweight abstraction for lazily evaluating a closure once and caching its result.  
It supports both infallible and fallible functions, with well-defined poisoning semantics to prevent use after panic or failed initialization.

## Example

```rust
use cached_fn::CachedFn;

// Infallible usage (compatible with FnOnce)
let mut called = false;
let mut c = CachedFn::new(|| {
    assert!(!called, "once");
    called = true;
    42
});
assert_eq!(*c.call(), 42);
assert_eq!(*c.call(), 42);
assert!(called);

// Fallible, poisoning version (compatible with FnOnce)
let mut p = CachedFn::new(|| -> Result<u32, &'static str> { Err("failed") });
assert!(p.poisoning_try_call().is_err());
assert!(p.is_poisoned());

// Fallible, retry-safe version (requires FnMut)
let mut attempts = 0;
let mut r = CachedFn::new(|| -> Result<u32, &'static str> {
    attempts += 1;
    if attempts < 3 {
        Err("not yet")
    } else if attempts == 3 {
        Ok(99)
    } else {
        panic!("too many");
    }
});
assert!(r.try_call().is_err());
assert!(r.try_call().is_err());
assert_eq!(*r.try_call().unwrap(), 99);
assert_eq!(*r.try_call().unwrap(), 99);
```
