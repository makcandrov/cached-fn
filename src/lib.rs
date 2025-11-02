#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![doc = include_str!("../README.md")]
#![no_std]

/// A lazily evaluated function that caches its result after the first call.
///
/// Once the function is called, its output is stored and subsequent calls will return the cached
/// value instead of recomputing it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CachedFn<F, Output>(CachedFnInner<F, Output>);

/// Internal state of a [`CachedFn`].
///
/// This enum tracks whether the function has been called, is still pending, or was poisoned due to
/// a failed or in-progress call.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CachedFnInner<F, Output> {
    /// Function has not been called yet.
    NotCalled(F),

    /// Function has been called and its result cached.
    Called(Output),

    /// Internal poisoned state, set temporarily during a call.
    ///
    /// This prevents reentrancy and ensures partial computation doesn't leak.
    Poisoned,
}

impl<F, Output> CachedFnInner<F, Output> {
    #[must_use]
    #[inline]
    fn into_not_called(self) -> Option<F> {
        match self {
            Self::NotCalled(f) => Some(f),
            _ => None,
        }
    }

    #[must_use]
    #[inline]
    const fn as_called_mut(&mut self) -> Option<&mut Output> {
        match self {
            Self::Called(output) => Some(output),
            _ => None,
        }
    }

    #[inline]
    const fn set_not_called(&mut self, f: F) -> Self {
        ::core::mem::replace(self, Self::NotCalled(f))
    }

    #[inline]
    const fn set_called(&mut self, output: Output) -> Self {
        ::core::mem::replace(self, Self::Called(output))
    }

    #[inline]
    const fn set_poisoned(&mut self) -> Self {
        ::core::mem::replace(self, Self::Poisoned)
    }
}

impl<F, Output> CachedFn<F, Output> {
    /// Creates a new [`CachedFn`] wrapping the given function.
    #[must_use]
    #[inline]
    pub const fn new(func: F) -> Self {
        Self(CachedFnInner::NotCalled(func))
    }

    /// Returns the cached output if the function has been called.
    #[must_use]
    #[inline]
    pub const fn as_called(&self) -> Option<&Output> {
        match &self.0 {
            CachedFnInner::Called(output) => Some(output),
            _ => None,
        }
    }

    /// Returns a reference to the underlying function if it hasn’t been called yet.
    #[must_use]
    #[inline]
    pub const fn as_not_called(&self) -> Option<&F> {
        match &self.0 {
            CachedFnInner::NotCalled(f) => Some(f),
            _ => None,
        }
    }

    /// Returns a mutable reference to the cached output if available.
    #[must_use]
    #[inline]
    pub const fn as_called_mut(&mut self) -> Option<&mut Output> {
        match &mut self.0 {
            CachedFnInner::Called(output) => Some(output),
            _ => None,
        }
    }

    /// Returns a mutable reference to the function if it hasn’t been called yet.
    #[must_use]
    #[inline]
    pub const fn as_not_called_mut(&mut self) -> Option<&mut F> {
        match &mut self.0 {
            CachedFnInner::NotCalled(f) => Some(f),
            _ => None,
        }
    }

    /// Converts into the cached value if available, otherwise returns `Err(self)`.
    #[inline]
    pub fn try_into_called(self) -> Result<Output, Self> {
        match self.0 {
            CachedFnInner::Called(output) => Ok(output),
            _ => Err(self),
        }
    }

    /// Converts into the underlying function if not yet called, otherwise returns `Err(self)`.
    #[inline]
    pub fn try_into_not_called(self) -> Result<F, Self> {
        match self.0 {
            CachedFnInner::NotCalled(f) => Ok(f),
            _ => Err(self),
        }
    }

    /// Returns `true` if the function has been called and its result cached.
    #[must_use]
    #[inline]
    pub const fn is_called(&self) -> bool {
        matches!(self.0, CachedFnInner::Called(_))
    }

    /// Returns `true` if the function has not yet been called.
    #[must_use]
    #[inline]
    pub const fn is_not_called(&self) -> bool {
        matches!(self.0, CachedFnInner::NotCalled(_))
    }

    /// Returns `true` if this [`CachedFn`] is in a poisoned state.
    #[must_use]
    #[inline]
    pub const fn is_poisoned(&self) -> bool {
        matches!(self.0, CachedFnInner::Poisoned)
    }
}

impl<F, Output> CachedFn<F, Output>
where
    F: FnOnce() -> Output,
{
    /// Calls the function if it hasn’t been called yet and caches its result.
    ///
    /// Returns a mutable reference to the cached result.
    ///
    /// # Panics
    ///
    /// Panics if the [`CachedFn`] is in a *poisoned* state.  
    /// This can occur if the wrapped function `f` previously panicked during execution.
    ///
    /// Once poisoned, the instance is considered unusable and further calls will panic.
    pub fn call(&mut self) -> &mut Output {
        let inner = &mut self.0;
        match inner {
            CachedFnInner::NotCalled(_) => {
                let f = inner.set_poisoned().into_not_called().unwrap();
                inner.set_called(f());
                inner.as_called_mut().unwrap()
            }
            CachedFnInner::Called(res) => res,
            CachedFnInner::Poisoned => panic!("poisoned"),
        }
    }

    /// Consumes the [`CachedFn`], calling the function if it hasn’t been called yet.
    ///
    /// # Panics
    ///
    /// Panics if the [`CachedFn`] is in a *poisoned* state.  
    /// This can occur if the wrapped function `f` previously panicked during execution.
    ///
    /// Once poisoned, the instance is considered unusable and further calls will panic.
    pub fn call_into(self) -> Output {
        match self.0 {
            CachedFnInner::NotCalled(f) => f(),
            CachedFnInner::Called(res) => res,
            CachedFnInner::Poisoned => panic!("poisoned"),
        }
    }
}

impl<F, Output, E> CachedFn<F, Output>
where
    F: FnOnce() -> Result<Output, E>,
{
    /// Calls the function and caches its result on success.
    ///
    /// This method executes the inner function `f` if it has not yet been called.
    /// On success, the computed value is stored internally, and a mutable reference to the cached
    /// result is returned.
    ///
    /// If the function returns an error, the [`CachedFn`] enters a **poisoned** state. Once
    /// poisoned, the instance must be **dropped** and never reused. This prevents further calls
    /// from observing or reusing potentially inconsistent or partially initialized state.
    ///
    /// Subsequent calls to a poisoned [`CachedFn`] will unconditionally panic.
    ///
    /// # Panics
    ///
    /// Panics if the [`CachedFn`] is in a *poisoned* state.  
    /// This can occur if:
    /// - The wrapped function `f` previously panicked during execution, or
    /// - A prior fallible call (via [`poisoning_try_call`](#method.poisoning_try_call)) returned an
    ///   error.
    ///
    /// Once poisoned, the instance is considered unusable and further calls will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// use cached_fn::CachedFn;
    ///
    /// let mut c = CachedFn::new(|| -> Result<u32, &'static str> { Err("failed to compute") });
    ///
    /// // First call returns an error and poisons the instance.
    /// assert!(c.poisoning_try_call().is_err());
    ///
    /// // Further calls will panic, as the instance is now poisoned.
    /// assert!(c.is_poisoned());
    /// // c.poisoning_try_call(); // <- panics
    /// ```
    pub fn poisoning_try_call(&mut self) -> Result<&mut Output, E> {
        let inner = &mut self.0;
        match inner {
            CachedFnInner::NotCalled(_) => {
                let f = inner.set_poisoned().into_not_called().unwrap();
                inner.set_called(f()?);
                Ok(inner.as_called_mut().unwrap())
            }
            CachedFnInner::Called(res) => Ok(res),
            CachedFnInner::Poisoned => panic!("poisoned"),
        }
    }

    /// Consumes the [`CachedFn`], calling the function if necessary and returning its result.
    ///
    /// # Panics
    ///
    /// Panics if the [`CachedFn`] is in a *poisoned* state.  
    /// This can occur if:
    /// - The wrapped function `f` previously panicked during execution, or
    /// - A prior fallible call (via [`poisoning_try_call`](#method.poisoning_try_call)) returned an
    ///   error.
    ///
    /// Once poisoned, the instance is considered unusable and further calls will panic.
    pub fn try_call_into(self) -> Result<Output, E> {
        match self.0 {
            CachedFnInner::NotCalled(f) => f(),
            CachedFnInner::Called(res) => Ok(res),
            CachedFnInner::Poisoned => panic!("poisoned"),
        }
    }

    /// Safely calls the function and returns a new [`CachedFn`] on success.
    ///
    /// Unlike [`poisoning_try_call`](#method.poisoning_try_call), this does **not** leave the
    /// instance in a poisoned state if the function returns an error.
    ///
    /// Instead, it consumes self, returning a new [`CachedFn`] containing the cached result on
    /// success, or the error directly on failure. This ensures that a failed call never leaves
    /// a poisoned instance behind.
    ///
    /// # Panics
    ///
    /// Panics if the [`CachedFn`] is in a *poisoned* state.  
    /// This can occur if:
    /// - The wrapped function `f` previously panicked during execution, or
    /// - A prior fallible call (via [`poisoning_try_call`](#method.poisoning_try_call)) returned an
    ///   error.
    ///
    /// Once poisoned, the instance is considered unusable and further calls will panic.
    pub fn safe_try_call(self) -> Result<Self, E> {
        match self.0 {
            CachedFnInner::NotCalled(f) => {
                let output = f()?;
                Ok(Self(CachedFnInner::Called(output)))
            }
            CachedFnInner::Called(_) => Ok(self),
            CachedFnInner::Poisoned => panic!("poisoned"),
        }
    }
}

impl<F, Output, E> CachedFn<F, Output>
where
    F: FnMut() -> Result<Output, E>,
{
    /// Attempts to call the function and caches its result on success.
    ///
    /// This method executes the inner function `f` if it has not yet been successfully called.
    /// If the call succeeds, the computed value is stored internally and a mutable reference to the
    /// cached result is returned.
    ///
    /// If the function returns an error, the [`CachedFn`] **remains callable** — it does **not**
    /// enter a poisoned state. Instead, the internal function `f` is restored, allowing future
    /// retry attempts.
    ///
    /// # Panics
    ///
    /// Panics if the [`CachedFn`] is in a *poisoned* state.  
    /// This can occur if:
    /// - The wrapped function `f` previously panicked during execution, or
    /// - A prior fallible call (via [`poisoning_try_call`](#method.poisoning_try_call)) returned an
    ///   error.
    ///
    /// Once poisoned, the instance is considered unusable and further calls will panic.
    ///
    /// # Examples
    ///
    /// ```
    /// use cached_fn::CachedFn;
    ///
    /// let mut tries = 0;
    /// let mut c = CachedFn::new(|| -> Result<u32, &'static str> {
    ///     tries += 1;
    ///     if tries < 3 {
    ///         Err("not yet")
    ///     } else {
    ///         Ok(42)
    ///     }
    /// });
    ///
    /// // Two failed attempts — instance is not poisoned and can be retried.
    /// assert!(c.try_call().is_err());
    /// assert!(c.try_call().is_err());
    ///
    /// // Third attempt succeeds and caches the result.
    /// assert_eq!(*c.try_call().unwrap(), 42);
    ///
    /// // Further calls return the cached value immediately.
    /// assert!(c.is_called());
    /// assert_eq!(*c.try_call().unwrap(), 42);
    /// ```
    pub fn try_call(&mut self) -> Result<&mut Output, E> {
        let inner = &mut self.0;
        match inner {
            CachedFnInner::NotCalled(_) => {
                let mut f = inner.set_poisoned().into_not_called().unwrap();
                match f() {
                    Ok(output) => {
                        inner.set_called(output);
                        Ok(inner.as_called_mut().unwrap())
                    }
                    Err(err) => {
                        inner.set_not_called(f);
                        Err(err)
                    }
                }
            }
            CachedFnInner::Called(res) => Ok(res),
            CachedFnInner::Poisoned => panic!("poisoned"),
        }
    }
}
