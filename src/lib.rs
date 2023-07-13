//! Rust port of Haskell's [`MVar`](https://hackage.haskell.org/package/base/docs/Control-Concurrent-MVar.html).
//!
//! An [`Mvar<T>`] is mutable location that is either empty or contais a value of type `T`.

#[cfg(all(feature = "shuttle", test))]
use shuttle::sync::{Condvar, Mutex, MutexGuard};
#[cfg(not(all(feature = "shuttle", test)))]
use std::sync::{Condvar, Mutex, MutexGuard};

use std::sync::PoisonError;

pub type LockError<'a, T> = PoisonError<MutexGuard<'a, Option<T>>>;

/// An [`Mvar`] (pronounced "em-var") is a synchronizing variable, used for communication between
/// concurrent threads. It can be thought as a box, which may be empty or full.
#[derive(Debug)]
pub struct Mvar<T> {
    value: Mutex<Option<T>>,
    full: Condvar,
    empty: Condvar,
}

impl<T> Default for Mvar<T> {
    /// Creates an empty Mvar
    fn default() -> Self {
        Self::empty()
    }
}

impl<T> Mvar<T> {
    /// Creates an empty Mvar
    pub fn empty() -> Self {
        Self {
            value: Mutex::default(),
            full: Condvar::default(),
            empty: Condvar::default(),
        }
    }

    /// Creates an Mvar which contains the value.
    pub fn new(value: T) -> Self {
        Self {
            value: Mutex::new(Some(value)),
            full: Condvar::default(),
            empty: Condvar::default(),
        }
    }

    pub fn is_empty(&self) -> Result<bool, LockError<T>> {
        Ok(self.value.lock()?.is_none())
    }

    pub fn take(&self) -> Result<T, LockError<T>> {
        let mut guard = self.value.lock()?;
        loop {
            if let Some(value) = guard.take() {
                self.empty.notify_one();
                return Ok(value);
            }
            guard = self.full.wait(guard)?;
        }
    }

    pub fn try_take(&self) -> Result<Option<T>, LockError<T>> {
        let mut guard = self.value.lock()?;
        let value = guard.take();
        if value.is_some() {
            self.empty.notify_one();
        }
        Ok(value)
    }

    pub fn put(&self, value: T) -> Result<(), LockError<T>> {
        let mut guard = self.value.lock()?;
        loop {
            if guard.is_none() {
                *guard = Some(value);
                self.full.notify_one();
                return Ok(());
            }
            guard = self.empty.wait(guard)?;
        }
    }

    pub fn try_put(&self, value: T) -> Result<bool, LockError<T>> {
        let mut guard = self.value.lock()?;
        if guard.is_some() {
            return Ok(false);
        }
        *guard = Some(value);
        self.full.notify_one();
        Ok(true)
    }

    pub fn swap(&self, value: T) -> Result<T, LockError<T>> {
        let mut guard = self.value.lock()?;
        let old_value = loop {
            if let Some(value) = guard.take() {
                break value;
            }
            guard = self.empty.wait(guard)?
        };
        *guard = Some(value);
        Ok(old_value)
    }
}

impl<T: Clone> Mvar<T> {
    pub fn read(&self) -> Result<T, LockError<T>> {
        let mut guard = self.value.lock()?;
        loop {
            if let Some(value) = guard.clone() {
                return Ok(value);
            }
            guard = self.full.wait(guard)?;
        }
    }

    pub fn try_read(&self) -> Result<Option<T>, LockError<T>> {
        let guard = self.value.lock()?;
        Ok(guard.clone())
    }
}

#[cfg(all(feature = "shuttle", test))]
mod tests {
    use super::*;

    use shuttle::{sync::Arc, thread};

    #[cfg(feature = "shuttle")]
    #[test]
    fn put_once() {
        shuttle::check_pct(
            || {
                let v = Arc::new(Mvar::default());
                assert!(v.is_empty().unwrap());
                thread::spawn({
                    let v = Arc::clone(&v);
                    move || {
                        v.put("x").unwrap();
                    }
                });
                assert_eq!(v.take().unwrap(), "x");
                assert!(v.is_empty().unwrap());
            },
            100,
            2,
        )
    }

    #[cfg(feature = "shuttle")]
    #[test]
    fn put_twice() {
        shuttle::check_pct(
            || {
                let v = Arc::new(Mvar::default());
                let thread = thread::spawn({
                    let v = Arc::clone(&v);
                    move || {
                        v.put("x").unwrap();
                        v.put("y").unwrap();
                    }
                });
                assert_eq!(v.take().unwrap(), "x");
                assert_eq!(v.take().unwrap(), "y");
                thread.join().unwrap();
            },
            100,
            2,
        )
    }
}
