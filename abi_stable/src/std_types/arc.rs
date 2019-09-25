/*!
Contains the ffi-safe equivalent of `std::sync::Arc`.
*/

use std::{
    borrow::{Borrow},
    marker::PhantomData, 
    mem::ManuallyDrop, 
    sync::Arc,
};

use core_extensions::prelude::*;

use crate::{
    abi_stability::StableAbi,
    pointer_trait::{
        CallReferentDrop, CanTransmuteElement,
        GetPointerKind,PK_SmartPointer,
    },
    prefix_type::{PrefixTypeTrait,WithMetadata},
    sabi_types::{Constructor,StaticRef},
    std_types::{RResult,utypeid::{UTypeId,new_utypeid}},
};


#[cfg(all(test,not(feature="only_new_tests")))]
mod test;

mod private {
    use super::*;

/**    
Ffi-safe version of ::std::sync::Arc<_>

# Example

Using an `RArc<RMutex<RVec<u32>>>` to get a list populated from multiple threads.

```
use abi_stable::{
    external_types::RMutex,
    std_types::{RArc,RVec},
};

use std::thread;

let arc=RArc::new(RMutex::new(RVec::new()));

{
    let arc2=RArc::clone(&arc);
    assert!( std::ptr::eq(&*arc,&*arc2) );
}

let mut guards=Vec::new();

for i in 0..10_u64 {
    let arc=RArc::clone(&arc);
    guards.push(thread::spawn(move||{
        for j in 0..100_u64{
            arc.lock().push(i*100+j);
        }
    }));
}

for guard in guards{
    guard.join().unwrap();
}

let mut vec=RArc::try_unwrap(arc)
    .ok()
    .expect("All the threads were joined,so this must be the only RArc")
    .into_inner();

vec.sort();

assert_eq!( vec, (0..1000).collect::<RVec<_>>() );


```

*/
    #[derive(StableAbi)]
    #[repr(C)]
    pub struct RArc<T> {
        data: *const T,
        vtable: StaticRef<ArcVtable<T>>,
        _marker: PhantomData<T>,
    }

    impl_from_rust_repr! {
        impl[T] From<Arc<T>> for RArc<T> {
            fn(this){
                let out = RArc {
                    data: Arc::into_raw(this),
                    vtable: WithMetadata::as_prefix(VTableGetter::LIB_VTABLE),
                    _marker: Default::default(),
                };
                out
            }
        }
    }

    unsafe impl<T> GetPointerKind for RArc<T>{
        type Kind=PK_SmartPointer;
    }

    unsafe impl<T, O> CanTransmuteElement<O> for RArc<T> {
        type TransmutedPtr = RArc<O>;
    }

    impl<T> RArc<T> {
        #[inline(always)]
        pub(super) fn data(&self) -> *const T {
            self.data
        }
        
        #[inline(always)]
        pub(super) unsafe fn data_mut(&mut self) -> *mut T {
            self.data as *mut T
        }

        #[inline]
        pub(crate) fn into_raw(self) -> *const T {
            let this = ManuallyDrop::new(self);
            this.data
        }

        #[inline(always)]
        pub(crate) fn vtable<'a>(&self) -> &'a ArcVtable<T> {
            self.vtable.get()
        }

        #[allow(dead_code)]
        #[cfg(test)]
        pub(super) fn set_vtable_for_testing(&mut self) {
            self.vtable = WithMetadata::as_prefix(
                VTableGetter::LIB_VTABLE_FOR_TESTING
            );
        }
    }
}

pub use self::private::RArc;

impl<T> RArc<T> {
    /// Constructs an `RArc` from a value.
    ///
    /// # Example
    ///
    /// ```
    /// use abi_stable::std_types::RArc;
    ///
    /// let arc=RArc::new(100);
    ///
    /// ```
    pub fn new(this: T) -> Self {
        Arc::new(this).into()
    }

    /// Converts this into an `Arc<T>`
    ///
    /// # Allocators
    ///
    /// The reason `RArc<T>` cannot always be converted to an `Arc<T>` is 
    /// because their allocators might be different.
    ///
    /// # When is T cloned
    ///
    /// `T` is cloned if the current dynamic_library/executable is 
    /// not the one that created the `RArc<T>`,
    /// and the strong count is greater than 1.
    ///
    /// # Example
    ///
    /// ```
    /// use abi_stable::std_types::RArc;
    /// use std::sync::Arc;
    ///
    /// let arc=RArc::new(100);
    ///
    /// assert_eq!( RArc::into_arc(arc), Arc::new(100) );
    ///
    /// ```
    pub fn into_arc(this: Self) -> Arc<T>
    where
        T: Clone,
    {
        let this_vtable =this.vtable();
        let other_vtable=WithMetadata::as_prefix(VTableGetter::LIB_VTABLE).get();
        if ::std::ptr::eq(this_vtable,other_vtable)||
            this_vtable.type_id()==other_vtable.type_id()
        {
            unsafe { Arc::from_raw(this.into_raw()) }
        } else {
            Self::try_unwrap(this)
                .unwrap_or_else(|x| T::clone(&x))
                .piped(Arc::new)
        }
    }

    /// Attempts to unwrap this `RArc<T>` into a `T`,
    /// returns Err(self) if the `RArc<T>`s strong count is greater than 1.
    ///
    /// # Example
    ///
    /// ```
    /// use abi_stable::std_types::RArc;
    /// 
    /// let arc0=RArc::new(100);
    /// assert_eq!( RArc::try_unwrap(arc0), Ok(100) );
    ///
    /// let arc1=RArc::new(100);
    /// let arc1_clone=RArc::clone(&arc1);
    /// assert_eq!( RArc::try_unwrap(arc1), Err(arc1_clone.clone()) );
    ///
    /// ```
    #[inline]
    pub fn try_unwrap(this: Self) -> Result<T, Self>{
        let vtable = this.vtable();
        unsafe{
            (vtable.try_unwrap())(this).into_result()
        }
    }

    /// Attempts to create a mutable reference to `T`,
    /// failing if `RArc<T>`s strong count is greater than 1.
    ///
    /// # Example
    ///
    /// ```
    /// use abi_stable::std_types::RArc;
    /// 
    /// let mut arc0=RArc::new(100);
    /// *RArc::get_mut(&mut arc0).unwrap()+=400;
    /// assert_eq!( *arc0, 500 );
    ///
    /// let mut arc1=RArc::new(100);
    /// let _arc1_clone=RArc::clone(&arc1);
    /// assert_eq!( RArc::get_mut(&mut arc1), None );
    ///
    /// ```
    #[inline]
    pub fn get_mut(this: &mut Self) -> Option<&mut T>{
        let vtable = this.vtable();
        unsafe{
            (vtable.get_mut())(this)
        }
    }

    /// Makes a mutable reference to `T`,
    /// if there are other `RArc<T>`s pointing to the same value,
    /// then `T` is cloned to ensure unique ownership of the value.
    ///
    /// # Postconditions
    ///
    /// After this call, the strong count of `this` will be 1,
    /// because either it was 1 before the call,
    /// or because a new `RArc<T>` was created to ensure unique ownership of `T`.
    ///
    /// # Example
    ///
    /// ```
    /// use abi_stable::std_types::RArc;
    /// 
    /// let mut arc0=RArc::new(100);
    /// *RArc::make_mut(&mut arc0)+=400;
    /// assert_eq!( *arc0, 500 );
    ///
    /// let mut arc1=RArc::new(100);
    /// let arc1_clone=RArc::clone(&arc1);
    /// *RArc::make_mut(&mut arc1)+=400;
    /// assert_eq!( *arc1, 500 );
    /// assert_eq!( *arc1_clone, 100 );
    ///
    /// ```
    #[inline]
    pub fn make_mut<'a>(this: &'a mut Self) -> &'a mut T 
    where T:Clone
    {
        // Workaround for non-lexical lifetimes not being smart enough 
        // to figure out that this borrow doesn't continue in the None branch.
        let unbounded_this=unsafe{ &mut *(this as *mut Self) };
        match Self::get_mut(unbounded_this) {
            Some(x)=>x,
            None=>{
                let new_arc=RArc::new((**this).clone());
                *this=new_arc;
                // This is fine,since this is a freshly created arc with a clone of the data.
                unsafe{
                    &mut *this.data_mut()
                }
            }
        }
    }

    /// Gets the number of RArc that point to the value.
    ///
    /// # Example
    ///
    /// ```
    /// use abi_stable::std_types::RArc;
    /// 
    /// let arc=RArc::new(0);
    /// assert_eq!( RArc::strong_count(&arc), 1 );
    ///
    /// let clone=RArc::clone(&arc);
    /// assert_eq!( RArc::strong_count(&arc), 2 );
    ///
    /// ```
    pub fn strong_count(this:&Self)->usize{
        let vtable = this.vtable();
        unsafe{
            vtable.strong_count()(this)
        }
    }

    /// Gets the number of std::sync::Weak that point to the value.
    ///
    /// # Example
    ///
    /// ```
    /// use abi_stable::std_types::RArc;
    /// 
    /// use std::sync::Arc;
    /// 
    /// let rustarc=Arc::new(0);
    /// let arc=RArc::from(rustarc.clone());
    /// assert_eq!( RArc::weak_count(&arc), 0 );
    /// 
    /// let weak_0=Arc::downgrade(&rustarc);
    /// assert_eq!( RArc::weak_count(&arc), 1 );
    ///
    /// let weak_1=Arc::downgrade(&rustarc);
    /// assert_eq!( RArc::weak_count(&arc), 2 );
    /// ```
    pub fn weak_count(this:&Self)->usize{
        let vtable = this.vtable();
        unsafe{
            vtable.weak_count()(this)
        }
    }
    
}


////////////////////////////////////////////////////////////////////


impl<T> Borrow<T> for RArc<T>{
    fn borrow(&self)->&T{
        self
    }
}


impl<T> AsRef<T> for RArc<T>{
    fn as_ref(&self)->&T{
        self
    }
}


////////////////////////////////////////////////////////////////////

impl<T> Default for RArc<T>
where
    T: Default,
{
    fn default() -> Self {
        RArc::new(T::default())
    }
}

impl<T> Clone for RArc<T> {
    fn clone(&self) -> Self {
        unsafe{
            (self.vtable().clone())(self)
        }
    }
}

impl_into_rust_repr! {
    impl[T] Into<Arc<T>> for RArc<T>
    where[
        T: Clone+StableAbi,
    ]{
        fn(this){
            Self::into_arc(this)
        }
    }
}

impl<T> Drop for RArc<T> {
    fn drop(&mut self) {
        // The layout of the RArc<_> won't change since it doesn't
        // actually support ?Sized types.
        unsafe {
            let vtable = self.vtable();
            (vtable.destructor())((self.data() as *const T).into(), CallReferentDrop::Yes);
        }
    }
}

shared_impls! {pointer
    mod=arc_impls
    new_type=RArc[][T],
    original_type=Arc,
}

unsafe impl<T> Sync for RArc<T> where T: Send + Sync {}

unsafe impl<T> Send for RArc<T> where T: Send + Sync {}

/////////////////////////////////////////////////////////

mod vtable_mod {
    use super::*;

    pub(super) struct VTableGetter<'a, T>(&'a T);

    impl<'a, T: 'a> VTableGetter<'a, T> {
        const DEFAULT_VTABLE:ArcVtableVal<T>=ArcVtableVal {
            type_id:Constructor( new_utypeid::<RArc<()>> ),
            destructor: destructor_arc::<T>,
            clone: clone_arc::<T>,
            get_mut: get_mut_arc::<T>,
            try_unwrap: try_unwrap_arc::<T>,
            strong_count: strong_count_arc::<T>,
            weak_count: weak_count_arc::<T>,
        };

        // The VTABLE for this type in this executable/library
        pub(super) const LIB_VTABLE: StaticRef<WithMetadata<ArcVtableVal<T>>> = unsafe{
            StaticRef::from_raw(&WithMetadata::new(
                PrefixTypeTrait::METADATA,
                Self::DEFAULT_VTABLE
            ))
        };

        #[allow(dead_code)]
        #[cfg(test)]
        pub(super) const LIB_VTABLE_FOR_TESTING: StaticRef<WithMetadata<ArcVtableVal<T>>> = unsafe{
            StaticRef::from_raw(&WithMetadata::new(
                PrefixTypeTrait::METADATA,
                ArcVtableVal{
                    type_id:Constructor( new_utypeid::<RArc<i32>> ),
                    ..Self::DEFAULT_VTABLE
                }
            ))
        };
    }

    #[derive(StableAbi)]
    #[repr(C)]
    #[sabi(kind(Prefix(prefix_struct="ArcVtable")))]
    #[sabi(missing_field(panic))]
    pub struct ArcVtableVal<T> {
        pub(super) type_id:Constructor<UTypeId>,
        pub(super) destructor: unsafe extern "C" fn(*const T, CallReferentDrop),
        pub(super) clone: unsafe extern "C" fn(&RArc<T>) -> RArc<T>,
        pub(super) get_mut: unsafe extern "C" fn(&mut RArc<T>) -> Option<&mut T>,
        pub(super) try_unwrap: unsafe extern "C" fn(RArc<T>) -> RResult<T, RArc<T>>,
        pub(super) strong_count: unsafe extern "C" fn(&RArc<T>) -> usize,
        #[sabi(last_prefix_field)]
        pub(super) weak_count:unsafe extern "C" fn(&RArc<T>) -> usize,
    }

    unsafe extern "C" fn destructor_arc<T>(this: *const T, call_drop: CallReferentDrop) {
        extern_fn_panic_handling! {no_early_return;
            if call_drop == CallReferentDrop::Yes {
                drop(Arc::from_raw(this));
            } else {
                drop(Arc::from_raw(this as *const ManuallyDrop<T>));
            }
        }
    }

    unsafe fn with_arc_ref<T,F,R>(this:&RArc<T>,f:F)->R
    where
        F:FnOnce(&Arc<T>)->R
    {
        let x=this.data();
        let x=Arc::from_raw(x);
        let x=ManuallyDrop::new(x);
        f(&x)
    }

    unsafe extern "C" fn clone_arc<T>(this: &RArc<T>) -> RArc<T> {
        with_arc_ref(this,|x| Arc::clone(&x).into() )
    }

    unsafe extern "C" fn get_mut_arc<'a,T>(this: &'a mut RArc<T>) -> Option<&'a mut T> {
        let arc=Arc::from_raw(this.data());
        let mut arc=ManuallyDrop::new(arc);
        // This is fine,since we are only touching the data afterwards,
        // which is guaranteed to have the 'a lifetime.
        let arc:&'a mut Arc<T>=&mut *(&mut *arc as *mut Arc<T>);
        Arc::get_mut(arc)
    }

    unsafe extern "C" fn try_unwrap_arc<T>(this: RArc<T>) -> RResult<T, RArc<T>> {
        this.into_raw()
            .piped(|x| Arc::from_raw(x))
            .piped(Arc::try_unwrap)
            .map_err(RArc::from)
            .into()
    }

    unsafe extern "C" fn strong_count_arc<T>(this: &RArc<T>) -> usize {
        with_arc_ref(this,|x| Arc::strong_count(&x) )
    }

    unsafe extern "C" fn weak_count_arc<T>(this: &RArc<T>) -> usize {
        with_arc_ref(this,|x| Arc::weak_count(&x) )
    }


}
use self::vtable_mod::{ArcVtable, VTableGetter};
