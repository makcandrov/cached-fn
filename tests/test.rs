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
