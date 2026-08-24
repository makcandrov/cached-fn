use cached_fn::CachedFn;

#[test]
fn test_cached_fn() {
    let mut x = 0usize;
    let f = || {
        x += 1;
        x + 1
    };
    let mut cached_f = CachedFn::<_, usize>::new(f);

    assert_eq!(*cached_f.call(), 2);
    assert_eq!(*cached_f.call(), 2);
    assert_eq!(*cached_f.call(), 2);
    assert_eq!(*cached_f.call(), 2);

    assert_eq!(x, 1);
}

#[test]
fn test_debug() {
    let mut cached_f = CachedFn::<_, usize>::new(|| 42);
    assert_eq!(format!("{cached_f:?}"), "CachedFn(<not called>)");

    cached_f.call();
    assert_eq!(format!("{cached_f:?}"), "CachedFn(42)");
    assert_eq!(format!("{cached_f:#?}"), "CachedFn(\n    42,\n)");

    let mut poisoned = CachedFn::<_, usize>::new(|| panic!("boom"));
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        poisoned.call();
    }));
    assert_eq!(format!("{poisoned:?}"), "CachedFn(<poisoned>)");
}
