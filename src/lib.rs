#![feature(dropck_eyepatch)]

use std::fmt::Debug;
use std::marker::PhantomData;
use std::ptr::NonNull;

pub struct Boks<T> {
    p: NonNull<T>,
    phantom: PhantomData<T>,
}

impl<T> Boks<T> {
    pub fn ne(t: T) -> Self {
        Self {
            // SAFETY: Box::into_raw always return a pointer
            p: unsafe { NonNull::new_unchecked(Box::into_raw(Box::new(t))) },
            phantom: PhantomData,
        }
    }
}

/// Without `#[may_dangle]`: the drop checker requires `T` to still be valid
/// when `Boks<T>::drop` runs, assuming the destructor might access `T`.
///
/// With `#[may_dangle]`: `T` is allowed to be logically dropped before `drop`,
/// because the destructor promises not to access `T`.
unsafe impl<#[may_dangle] T> Drop for Boks<T> {
    fn drop(&mut self) {
        // SAFETY: p was constructed from a box and has not been freed since.
        unsafe {
            Box::from_raw(self.p.as_ptr());
        }
    }
}

impl<T> std::ops::Deref for Boks<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: is valid since it was constructed from a valid T, and turned into a pointer
        // through Box which creates aligned pointer and hasn't been freed as self is not dropped
        unsafe { &*self.p.as_ref() }
    }
}

impl<T> std::ops::DerefMut for Boks<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: is valid since it was constructed from a valid T, and turned into a pointer
        // through Box which creates aligned pointer and hasn't been freed as self is not dropped
        // As we have a mut reference means no other immutable and mutable reference given.
        unsafe { &mut *self.p.as_mut() }
    }
}

pub struct Oisann<T: Debug>(T);

impl<T: Debug> Oisann<T> {
    pub fn ne(t: T) -> Self {
        Oisann(t)
    }
}

impl<T: Debug> Drop for Oisann<T> {
    fn drop(&mut self) {
        println!("{:?}", self.0);
    }
}

// If we use T here it will assume it drops the T here
// which it does not. So using fn() -> T keeps it covariant
// and also does not check for drop of T
pub struct Empty<T>(PhantomData<fn() -> T>);

impl<T> Iterator for Empty<T> {
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{Boks, Oisann};

    #[test]
    fn it_works() {
        let x = 42;
        let b = Boks::ne(x);
        println!("{:?}", *b);
    }

    #[test]
    fn try_mutable_param() {
        let mut y = 42;
        // This work:
        // let stdb = Box::new(&mut y);
        // println!("{}", y);

        let b = Boks::ne(&mut y);
        // As Drop for Boks (with generic param) is implemented, so compiler assumes it access the
        // inner value T as dropped so can't borrow immutably or mutably once done in Boks.
        println!("{}", y);
    }

    #[test]
    fn drop_boks_with_oiasnn() {
        let mut z = 42;
        // This does not compile
        // let b = Box::new(Oisann::ne(&mut z));
        // println!("{:?}", z);

        // But our code does, hence need to add PhantomData to tell
        // we are not accessing but dropping the inner value.
        let b = Boks::ne(Oisann::ne(&mut z));
        // Now with phantomData this won't compile as we said we
        // will drop the value, so look into the inner type drop whether
        // it access and if yes, make it not compile.
        // println!("{:?}", z);
    }

    #[test]
    fn boks_without_non_null_is_invariant() {
        // Boks has *mut T which is invariant

        let mut s = String::from("hei");
        let mut stdb1: Box<&str> = Box::new(&*s);
        let mut stdb2: Box<&'static str> = Box::new("hello");
        stdb1 = stdb2;

        let mut s = String::from("hei");
        let mut stdb1: Boks<&str> = Boks::ne(&*s);
        let mut stdb2: Boks<&'static str> = Boks::ne("hello");
        // This does not work as *mut is invariant so we can fix it by using NonNull
        // which is covariant
        stdb1 = stdb2;
    }
}
