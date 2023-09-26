use parking_lot::Mutex;
use std::iter::FromIterator;
use std::mem::{forget, ManuallyDrop};
use std::ops::{Deref, DerefMut};

pub type Stack<T> = Vec<T>;

pub struct Pool<T> {
    objects: Mutex<Stack<T>>,
}

impl<T> Pool<T> {
    #[inline]
    pub fn new<F>(cap: usize, init: F) -> Pool<T>
    where
        F: Fn() -> T,
    {
        Pool {
            objects: Mutex::new((0..cap).into_iter().map(|_| init()).collect()),
        }
    }

    #[inline]
    pub fn from_vec(v: Vec<T>) -> Pool<T> {
        Pool {
            objects: Mutex::new(v),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.objects.lock().len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.objects.lock().is_empty()
    }

    #[inline]
    pub fn try_pull(&self) -> Option<Reusable<T>> {
        self.objects
            .lock()
            .pop()
            .map(|data| Reusable::new(self, data))
    }

    #[inline]
    pub fn pull<F: Fn() -> T>(&self, fallback: F) -> Reusable<T> {
        self.try_pull()
            .unwrap_or_else(|| Reusable::new(self, fallback()))
    }

    #[inline]
    pub fn attach(&self, t: T) {
        self.objects.lock().push(t)
    }
}

impl<T> FromIterator<T> for Pool<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            objects: Mutex::new(iter.into_iter().collect()),
        }
    }
}

pub struct Reusable<'a, T> {
    pool: &'a Pool<T>,
    data: ManuallyDrop<T>,
}

impl<'a, T> Reusable<'a, T> {
    #[inline]
    pub fn new(pool: &'a Pool<T>, t: T) -> Self {
        Self {
            pool,
            data: ManuallyDrop::new(t),
        }
    }

    #[inline]
    pub fn detach(mut self) -> (&'a Pool<T>, T) {
        let ret = unsafe { (self.pool, self.take()) };
        forget(self);
        ret
    }

    unsafe fn take(&mut self) -> T {
        ManuallyDrop::take(&mut self.data)
    }
}

impl<'a, T> Deref for Reusable<'a, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T> DerefMut for Reusable<'a, T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<'a, T> Drop for Reusable<'a, T> {
    #[inline]
    fn drop(&mut self) {
        unsafe { self.pool.attach(self.take()) }
    }
}
