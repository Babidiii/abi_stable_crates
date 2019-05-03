/*!
Contains the `DynTrait` type,and related traits/type aliases.
*/

use std::{
    ops::DerefMut,
    marker::PhantomData,
    mem::ManuallyDrop,
    ptr,
};

use serde::{de, ser, Deserialize, Deserializer};

#[allow(unused_imports)]
use core_extensions::{prelude::*, ResultLike};

use crate::{
    abi_stability::SharedStableAbi,
    pointer_trait::{StableDeref, TransmuteElement},
    marker_type::ErasedObject, 
    std_types::{RBox, RCow, RStr,RVec},
};

#[allow(unused_imports)]
use crate::std_types::Tuple2;

use super::*;
use super::{
    c_functions::adapt_std_fmt,
    trait_objects::*,
    vtable::{GetVtable, VTable},
    traits::InterfaceFor,
};


#[cfg(test)]
mod tests;

mod priv_ {
    use super::*;


    /**

DynTrait implements ffi-safe trait objects,for a selection of traits.

# Passing opaque values around with `DynTrait<_>`

One can pass non-StableAbi types around by using type erasure,using this type.

It generally looks like `DynTrait<'borrow,Pointer<()>,Interface>`,where:

- `'borrow` is the borrow that the type that was erased had.

- `Pointer` is some `pointer_trait::StableDeref` pointer type.

- `Interface` is an `InterfaceType`,which describes what traits are 
    required when constructing the `DynTrait<_>` and which ones it implements.

`trait InterfaceType` allows describing which traits are required 
when constructing a `DynTrait<_>`,and which ones it implements.

### Construction

To construct a `DynTrait<_>` one can use these associated functions:
    
- from_value:
    Can be constructed from the value directly.
    Requires a value that implements ImplType.
    
- from_ptr:
    Can be constructed from a pointer of a value.
    Requires a value that implements ImplType.
    
- from_any_value:
    Can be constructed from the value directly.Requires a `'static` value.
    
- from_any_ptr
    Can be constructed from a pointer of a value.Requires a `'static` value.

- from_borrowing_value:
    Can be constructed from the value directly.Cannot unerase the DynTrait afterwards.
    
- from_borrowing_ptr
    Can be constructed from a pointer of a value.Cannot unerase the DynTrait afterwards.

### Trait object

`DynTrait<'borrow,Pointer<()>,Interface>` 
can be used as a trait object for any combination of 
the traits listed bellow.

These are the traits:

- Send

- Sync

- Iterator

- DoubleEndedIterator

- Clone 

- Display 

- Debug 

- Default: Can be called as an inherent method.

- Eq 

- PartialEq 

- Ord 

- PartialOrd 

- Hash 

- serde::Deserialize:
    first deserializes from a string,and then calls the objects' Deserialize impl.

- serde::Serialize:
    first calls the objects' Deserialize impl,then serializes that as a string.

### Deconstruction

`DynTrait<_>` can then be unwrapped into a concrete type,
within the same dynamic library/executable that constructed it,
using these (fallible) conversion methods:

- into_unerased:
    Unwraps into a pointer to `T`.
    Where `DynTrait<P<()>,Interface>`'s 
        Interface must equal `<T as ImplType>::Interface`

- as_unerased:
    Unwraps into a `&T`.
    Where `DynTrait<P<()>,Interface>`'s 
        Interface must equal `<T as ImplType>::Interface`

- as_unerased_mut:
    Unwraps into a `&mut T`.
    Where `DynTrait<P<()>,Interface>`'s 
        Interface must equal `<T as ImplType>::Interface`

- into_any_unerased:Unwraps into a pointer to `T`.Requires `T:'static`.

- as_any_unerased:Unwraps into a `&T`.Requires `T:'static`.

- as_any_unerased_mut:Unwraps into a `&mut T`.Requires `T:'static`.


Note that `DynTrait` cannot be converted back if it was created 
using `DynTrait::from_borrowing_*`.

# Passing DynTrait between dynamic libraries

Passing DynTrait between sibling dynamic libraries 
(as in between the dynamic libraries directly loaded by the same binary/dynamic library)
may cause the program to abort at runtime with an error message stating that 
the trait is not implemented for the specific interface.

This can only happen if you are passing DynTrait between sibling dynamic libraries though,
passing DynTrait instantiated in parent/child dynamic library to the parent/child 
should not cause an abort,it would be a bug.

```text
        binary
  _________|___________
lib0      lib1      lib2
  |         |         |
lib00    lib10      lib20
```

In this diagram passing a DynTrait constructed in lib00 to anything other than 
the binary or lib0 will cause the abort to happen if:

- The InterfaceType requires extra traits in the version of the Interface
    that lib1 and lib2 know about (that the binary does not require).

- lib1 or lib2 attempt to call methods that require the traits that were added 
    to the InterfaceType,in versions of that interface that only they know about.


# Example 

The primary example using `DynTrait<_>` is in the readme.

Readme is in 
[the repository for this crate](https://github.com/rodrimati1992/abi_stable_crates),
[crates.io](https://crates.io/crates/abi_stable),
[lib.rs](https://lib.rs/crates/abi_stable).

    
    */
    #[repr(C)]
    #[derive(StableAbi)]
    #[sabi(
        inside_abi_stable_crate,
        prefix_bound="I:InterfaceConstsBound<'borr>",
        bound="<I as SharedStableAbi>::StaticEquivalent:InterfaceConstsBound<'static>",
        bound="VTable<'borr,P,I>:SharedStableAbi",
        tag="<I as InterfaceConstsBound<'borr>>::TAG",
    )]
    pub struct DynTrait<'borr,P,I> 
    where I:InterfaceConstsBound<'borr>
    {
        pub(super) object: ManuallyDrop<P>,
        vtable: *const VTable<'borr,P,I>,
        _marker:PhantomData<extern fn()->Tuple2<I,RStr<'borr>>>
    }

    impl DynTrait<'static,&'static (),()> {
        /// Constructors the `DynTrait<_>` from an ImplType implementor.
        ///
        /// Use this whenever possible instead of `from_any_value`,
        /// because it produces better error messages when unerasing the `DynTrait<_>`
        pub fn from_value<T>(object: T) -> DynTrait<'static,RBox<()>,T::Interface>
        where
            T: ImplType,
            T::Interface:InterfaceConstsBound<'static>,
            T: GetVtable<'static,T,RBox<()>,RBox<T>,<T as ImplType>::Interface>,
        {
            let object = RBox::new(object);
            DynTrait::from_ptr(object)
        }

        /// Constructors the `DynTrait<_>` from a pointer to an ImplType implementor.
        ///
        /// Use this whenever possible instead of `from_any_ptr`,
        /// because it produces better error messages when unerasing the `DynTrait<_>`
        pub fn from_ptr<P, T>(object: P) -> DynTrait<'static,P::TransmutedPtr,T::Interface>
        where
            T: ImplType,
            T::Interface:InterfaceConstsBound<'static>,
            T: GetVtable<'static,T,P::TransmutedPtr,P,<T as ImplType>::Interface>,
            P: StableDeref<Target = T>+TransmuteElement<()>,
        {
            DynTrait {
                object: unsafe{
                    // The lifetime here is 'static,so it's fine to erase the type.
                    ManuallyDrop::new(object.transmute_element(<()>::T))
                },
                vtable: T::get_vtable(),
                _marker:PhantomData,
            }
        }

        /// Constructors the `DynTrait<_>` from a type that doesn't implement `ImplType`.
        pub fn from_any_value<T,I>(object: T,interface:I) -> DynTrait<'static,RBox<()>,I>
        where
            T:'static,
            I:InterfaceConstsBound<'static>,
            InterfaceFor<T,I,True> : GetVtable<'static,T,RBox<()>,RBox<T>,I>,
        {
            let object = RBox::new(object);
            DynTrait::from_any_ptr(object,interface)
        }

        /// Constructors the `DynTrait<_>` from a pointer to a 
        /// type that doesn't implement `ImplType`.
        pub fn from_any_ptr<P, T,I>(
            object: P,
            _interface:I
        ) -> DynTrait<'static,P::TransmutedPtr,I>
        where
            I:InterfaceConstsBound<'static>,
            T:'static,
            InterfaceFor<T,I,True>: GetVtable<'static,T,P::TransmutedPtr,P,I>,
            P: StableDeref<Target = T>+TransmuteElement<()>,
        {
            DynTrait {
                object: unsafe{
                    // The lifetime here is 'static,so it's fine to erase the type.
                    ManuallyDrop::new(object.transmute_element(<()>::T))
                },
                vtable: <InterfaceFor<T,I,True>>::get_vtable(),
                _marker:PhantomData,
            }
        }
        
        /// Constructors the `DynTrait<_>` from a value.
        ///
        /// Cannot unerase the DynTrait afterwards.
        pub fn from_borrowing_value<'borr,T,I>(
            object: T,
            interface:I,
        ) -> DynTrait<'borr,RBox<()>,I>
        where
            T:'borr,
            I:InterfaceConstsBound<'borr>,
            InterfaceFor<T,I,False> : GetVtable<'borr,T,RBox<()>,RBox<T>,I>,
        {
            let object = RBox::new(object);
            DynTrait::from_borrowing_ptr(object,interface)
        }

        /// Constructors the `DynTrait<_>` from a pointer to the erased type.
        ///
        /// Cannot unerase the DynTrait afterwards.
        pub fn from_borrowing_ptr<'borr,P, T,I>(
            object: P,
            _interface:I
        ) -> DynTrait<'borr,P::TransmutedPtr,I>
        where
            T:'borr,
            I:InterfaceConstsBound<'borr>,
            InterfaceFor<T,I,False>: GetVtable<'borr,T,P::TransmutedPtr,P,I>,
            P: StableDeref<Target = T>+TransmuteElement<()>,
        {
            DynTrait {
                object: unsafe{
                    // The lifetime here is 'static,so it's fine to erase the type.
                    ManuallyDrop::new(object.transmute_element(<()>::T))
                },
                vtable: <InterfaceFor<T,I,False>>::get_vtable(),
                _marker:PhantomData,
            }
        }
    }


    impl<P,I> DynTrait<'static,P,I> 
    where 
        I: InterfaceConstsBound<'static>
    {
        /// Allows checking whether 2 `DynTrait<_>`s have a value of the same type.
        ///
        /// Notes:
        ///
        /// - Types from different dynamic libraries/executables are 
        /// never considered equal.
        ///
        /// - `DynTrait`s constructed using `DynTrait::from_borrowing_*`
        /// are never considered to wrap the same type.
        pub fn is_same_type<Other,I2>(&self,other:&DynTrait<'static,Other,I2>)->bool
        where I2:InterfaceConstsBound<'static>
        {
            self.vtable_address()==other.vtable_address()||
            self.vtable().type_info().is_compatible(other.vtable().type_info())
        }
    }

    impl<'borr,P,I> DynTrait<'borr,P,I> 
    where 
        I: InterfaceConstsBound<'borr>
    {
        pub(super) fn vtable<'a>(&self) -> &'a VTable<'borr,P,I>{
            unsafe {
                &*self.vtable
            }
        }

        pub(super)fn vtable_address(&self) -> usize {
            self.vtable as usize
        }

        pub(super) fn as_abi(&self) -> &ErasedObject
        where
            P: Deref,
        {
            self.object()
        }

        #[allow(dead_code)]
        pub(super) fn as_abi_mut(&mut self) -> &mut ErasedObject
        where
            P: DerefMut,
        {
            self.object_mut()
        }

        /// Returns the address of the wrapped object.
        ///
        /// This will not change between calls for the same `DynTrait<_>`.
        pub fn object_address(&self) -> usize
        where
            P: Deref,
        {
            self.object() as *const ErasedObject as usize
        }

        pub(super) fn object(&self) -> &ErasedObject
        where
            P: Deref,
        {
            unsafe { self.object_as() }
        }
        pub(super) fn object_mut(&mut self) -> &mut ErasedObject
        where
            P: DerefMut,
        {
            unsafe { self.object_as_mut() }
        }

        unsafe fn object_as<T>(&self) -> &T
        where
            P: Deref,
        {
            &*((&**self.object) as *const P::Target as *const T)
        }
        unsafe fn object_as_mut<T>(&mut self) -> &mut T
        where
            P: DerefMut,
        {
            &mut *((&mut **self.object) as *mut P::Target as *mut T)
        }
    }

    impl<'borr,P,I> DynTrait<'borr,P,I> 
    where 
        I: InterfaceConstsBound<'borr>
    {
        /// The uid in the vtable has to be the same as the one for T,
        /// otherwise it was not created from that T in the library that declared the opaque type.
        pub(super) fn check_same_destructor_opaque<A,T>(&self) -> Result<(), UneraseError>
        where
            P: TransmuteElement<T>,
            A: GetVtable<'borr,T,P,P::TransmutedPtr,I>,
        {
            let t_vtable:&VTable<'borr,P,I> = A::get_vtable();
            if self.vtable_address() == t_vtable as *const _ as usize
                || self.vtable().type_info().is_compatible(t_vtable.type_info())
            {
                Ok(())
            } else {
                Err(UneraseError {
                    expected_vtable_address: t_vtable as *const _ as usize,
                    expected_type_info:t_vtable.type_info(),
                    found_vtable_address: self.vtable as usize,
                    found_type_info:self.vtable().type_info(),
                })
            }
        }

        /// Unwraps the `DynTrait<_>` into a pointer of 
        /// the concrete type that it was constructed with.
        ///
        /// T is required to implement ImplType.
        ///
        /// # Errors
        ///
        /// This will return an error in any of these conditions:
        ///
        /// - It is called in a dynamic library/binary outside
        /// the one from which this `DynTrait<_>` was constructed.
        ///
        /// - The DynTrait was constructed using a `from_borrowing_*` method
        ///
        /// - `T` is not the concrete type this `DynTrait<_>` was constructed with.
        ///
        pub fn into_unerased<T>(self) -> Result<P::TransmutedPtr, UneraseError>
        where
            P: TransmuteElement<T>,
            P::Target:Sized,
            T: ImplType + GetVtable<'borr,T,P,P::TransmutedPtr,I>,
        {
            self.check_same_destructor_opaque::<T,T>()?;
            unsafe { 
                let this=ManuallyDrop::new(self);
                Ok(ptr::read(&*this.object).transmute_element(T::T)) 
            }
        }

        /// Unwraps the `DynTrait<_>` into a reference of 
        /// the concrete type that it was constructed with.
        ///
        /// T is required to implement ImplType.
        ///
        /// # Errors
        ///
        /// This will return an error in any of these conditions:
        ///
        /// - It is called in a dynamic library/binary outside
        /// the one from which this `DynTrait<_>` was constructed.
        ///
        /// - The DynTrait was constructed using a `from_borrowing_*` method
        ///
        /// - `T` is not the concrete type this `DynTrait<_>` was constructed with.
        ///
        pub fn as_unerased<T>(&self) -> Result<&T, UneraseError>
        where
            P: Deref + TransmuteElement<T>,
            T: ImplType + GetVtable<'borr,T,P,P::TransmutedPtr,I>,
        {
            self.check_same_destructor_opaque::<T,T>()?;
            unsafe { Ok(self.object_as()) }
        }

        /// Unwraps the `DynTrait<_>` into a mutable reference of 
        /// the concrete type that it was constructed with.
        ///
        /// T is required to implement ImplType.
        ///
        /// # Errors
        ///
        /// This will return an error in any of these conditions:
        ///
        /// - It is called in a dynamic library/binary outside
        /// the one from which this `DynTrait<_>` was constructed.
        ///
        /// - The DynTrait was constructed using a `from_borrowing_*` method
        ///
        /// - `T` is not the concrete type this `DynTrait<_>` was constructed with.
        ///
        pub fn as_unerased_mut<T>(&mut self) -> Result<&mut T, UneraseError>
        where
            P: DerefMut + TransmuteElement<T>,
            T: ImplType + GetVtable<'borr,T,P,P::TransmutedPtr,I>,
        {
            self.check_same_destructor_opaque::<T,T>()?;
            unsafe { Ok(self.object_as_mut()) }
        }


        /// Unwraps the `DynTrait<_>` into a pointer of 
        /// the concrete type that it was constructed with.
        ///
        /// T is required to not borrows anything.
        ///
        /// # Errors
        ///
        /// This will return an error in any of these conditions:
        ///
        /// - It is called in a dynamic library/binary outside
        /// the one from which this `DynTrait<_>` was constructed.
        ///
        /// - The DynTrait was constructed using a `from_borrowing_*` method
        ///
        /// - `T` is not the concrete type this `DynTrait<_>` was constructed with.
        ///
        pub fn into_any_unerased<T>(self) -> Result<P::TransmutedPtr, UneraseError>
        where
            T:'static,
            P: TransmuteElement<T>,
            P::Target:Sized,
            Self:DynTraitBound<'borr>,
            InterfaceFor<T,I,True>: GetVtable<'borr,T,P,P::TransmutedPtr,I>,
        {
            self.check_same_destructor_opaque::<InterfaceFor<T,I,True>,T>()?;
            unsafe {
                unsafe { 
                    let this=ManuallyDrop::new(self);
                    Ok(ptr::read(&*this.object).transmute_element(T::T)) 
                }
            }
        }

        /// Unwraps the `DynTrait<_>` into a reference of 
        /// the concrete type that it was constructed with.
        ///
        /// T is required to not borrows anything.
        ///
        /// # Errors
        ///
        /// This will return an error in any of these conditions:
        ///
        /// - It is called in a dynamic library/binary outside
        /// the one from which this `DynTrait<_>` was constructed.
        ///
        /// - The DynTrait was constructed using a `from_borrowing_*` method
        ///
        /// - `T` is not the concrete type this `DynTrait<_>` was constructed with.
        ///
        pub fn as_any_unerased<T>(&self) -> Result<&T, UneraseError>
        where
            T:'static,
            P: Deref + TransmuteElement<T>,
            Self:DynTraitBound<'borr>,
            InterfaceFor<T,I,True>: GetVtable<'borr,T,P,P::TransmutedPtr,I>,
        {
            self.check_same_destructor_opaque::<InterfaceFor<T,I,True>,T>()?;
            unsafe { Ok(self.object_as()) }
        }

        /// Unwraps the `DynTrait<_>` into a mutable reference of 
        /// the concrete type that it was constructed with.
        ///
        /// T is required to not borrows anything.
        ///
        /// # Errors
        ///
        /// This will return an error in any of these conditions:
        ///
        /// - It is called in a dynamic library/binary outside
        /// the one from which this `DynTrait<_>` was constructed.
        ///
        /// - The DynTrait was constructed using a `from_borrowing_*` method
        ///
        /// - `T` is not the concrete type this `DynTrait<_>` was constructed with.
        ///
        pub fn as_any_unerased_mut<T>(&mut self) -> Result<&mut T, UneraseError>
        where
            P: DerefMut + TransmuteElement<T>,
            Self:DynTraitBound<'borr>,
            InterfaceFor<T,I,True>: GetVtable<'borr,T,P,P::TransmutedPtr,I>,
        {
            self.check_same_destructor_opaque::<InterfaceFor<T,I,True>,T>()?;
            unsafe { Ok(self.object_as_mut()) }
        }

    }

    impl<'borr,P,I> DynTrait<'borr,P,I> 
    where 
        I:InterfaceConstsBound<'borr>
    {
        /// Constructs a DynTrait<P,I> wrapping a `P`,using the same vtable.
        /// `P` must come from a function in the vtable,
        /// to ensure that it is compatible with the functions in it.
        pub(super) fn from_new_ptr(&self, object: P) -> Self {
            Self {
                object:ManuallyDrop::new(object),
                vtable: self.vtable,
                _marker:PhantomData,
            }
        }

        /// Constructs a `DynTrait<P,I>` with the default value for `P`.
        pub fn default(&self) -> Self
        where
            P: Deref,
            I: InterfaceType<Default = True>,
        {
            let new = self.vtable().default_ptr()();
            self.from_new_ptr(new)
        }

        /// It serializes a `DynTrait<_>` into a string by using 
        /// `<ConcreteType as SerializeImplType>::serialize_impl`.
        pub fn serialized<'a>(&'a self) -> Result<RCow<'a, RStr<'a>>, RBoxError>
        where
            P: Deref,
            I: InterfaceType<Serialize = True>,
        {
            self.vtable().serialize()(self.as_abi()).into_result()
        }

        /// Deserializes a string into a `DynTrait<_>`,by using 
        /// `<I as DeserializeOwnedInterface>::deserialize_impl`.
        pub fn deserialize_owned_from_str(s: &str) -> Result<Self, RBoxError>
        where
            P: 'borr+Deref,
            I: DeserializeOwnedInterface<'borr,Deserialize = True, Deserialized = Self>,
        {
            s.piped(RStr::from).piped(I::deserialize_impl)
        }

        /// Deserializes a `&'borr str` into a `DynTrait<'borr,_>`,by using 
        /// `<I as DeserializeBorrowedInterface<'borr>>::deserialize_impl`.
        pub fn deserialize_borrowing_from_str(s: &'borr str) -> Result<Self, RBoxError>
        where
            P: 'borr+Deref,
            I: DeserializeBorrowedInterface<'borr,Deserialize = True, Deserialized = Self>,
        {
            s.piped(RStr::from).piped(I::deserialize_impl)
        }
    }

    impl<'borr,P,I> Drop for DynTrait<'borr,P,I>
    where I:InterfaceConstsBound<'borr>
    {
        fn drop(&mut self){
            unsafe{
                let vtable=&*self.vtable;
                vtable.drop_ptr()(&mut *self.object);
            }
        }
    }

}

pub use self::priv_::DynTrait;

impl<'borr,P, I> Clone for DynTrait<'borr,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'borr,Clone = True>,
{
    fn clone(&self) -> Self {
        let vtable = self.vtable();
        let new = vtable.clone_ptr()(&*self.object);
        self.from_new_ptr(new)
    }
}

impl<'borr,P, I> Display for DynTrait<'borr,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'borr,Display = True>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        adapt_std_fmt::<ErasedObject>(self.object(), self.vtable().display(), f)
    }
}

impl<'borr,P, I> Debug for DynTrait<'borr,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'borr,Debug = True>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        adapt_std_fmt::<ErasedObject>(self.object(), self.vtable().debug(), f)
    }
}

/**
First it serializes a `DynTrait<_>` into a string by using 
<ConcreteType as SerializeImplType>::serialize_impl,
then it serializes the string.

*/
/// ,then it .
impl<'borr,P, I> Serialize for DynTrait<'borr,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'borr,Serialize = True>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.vtable().serialize()(self.as_abi())
            .into_result()
            .map_err(ser::Error::custom)?
            .serialize(serializer)
    }
}

/// First it Deserializes a string,then it deserializes into a 
/// `DynTrait<_>`,by using `<I as DeserializeOwnedInterface>::deserialize_impl`.
impl<'de,'borr:'de, P, I> Deserialize<'de> for DynTrait<'borr,P,I>
where
    P: Deref+'borr,
    I: InterfaceConstsBound<'borr>,
    I: DeserializeOwnedInterface<'borr,Deserialize = True, Deserialized = Self>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        I::deserialize_impl(RStr::from(&*s)).map_err(de::Error::custom)
    }
}

impl<P, I> Eq for DynTrait<'static,P,I>
where
    Self: PartialEq,
    P: Deref,
    I: InterfaceConstsBound<'static,Eq = True>,
{
}

impl<P, I> PartialEq for DynTrait<'static,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'static,PartialEq = True>,
{
    fn eq(&self, other: &Self) -> bool {
        // unsafe: must check that the vtable is the same,otherwise return a sensible value.
        if !self.is_same_type(other) {
            return false;
        }

        self.vtable().partial_eq()(self.as_abi(), other.as_abi())
    }
}

impl<P, I> Ord for DynTrait<'static,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'static,Ord = True>,
    Self: PartialOrd + Eq,
{
    fn cmp(&self, other: &Self) -> Ordering {
        // unsafe: must check that the vtable is the same,otherwise return a sensible value.
        if !self.is_same_type(other) {
            return self.vtable_address().cmp(&other.vtable_address());
        }

        self.vtable().cmp()(self.as_abi(), other.as_abi()).into()
    }
}

impl<P, I> PartialOrd for DynTrait<'static,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'static,PartialOrd = True>,
    Self: PartialEq,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // unsafe: must check that the vtable is the same,otherwise return a sensible value.
        if !self.is_same_type(other) {
            return Some(self.vtable_address().cmp(&other.vtable_address()));
        }

        self.vtable().partial_cmp()(self.as_abi(), other.as_abi())
            .map(IntoReprRust::into_rust)
            .into()
    }
}

impl<'borr,P, I> Hash for DynTrait<'borr,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'borr,Hash = True>,
{
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.vtable().hash()(self.as_abi(), HasherObject::new(state))
    }
}


impl<'borr,P, I,Item> Iterator for DynTrait<'borr,P,I>
where
    P: DerefMut,
    I: InterfaceConstsBound<'borr,Iterator = True,IteratorItem=Item>,
{
    type Item=Item;

    fn next(&mut self)->Option<Item>{
        let vtable=self.vtable();
        (vtable.iter().next)(self.as_abi_mut()).into_rust()
    }

    fn nth(&mut self,nth:usize)->Option<Item>{
        let vtable=self.vtable();
        (vtable.iter().nth)(self.as_abi_mut(),nth).into_rust()
    }

    fn size_hint(&self)->(usize,Option<usize>){
        let vtable=self.vtable();
        let tuple=(vtable.iter().size_hint)(self.as_abi()).into_rust();
        (tuple.0,tuple.1.into_rust())
    }

    fn count(mut self)->usize{
        let vtable=self.vtable();
        (vtable.iter().count)(self.as_abi_mut())
    }

    fn last(mut self)->Option<Item>{
        let vtable=self.vtable();
        (vtable.iter().last)(self.as_abi_mut()).into_rust()
    }
}


impl<'borr,P, I,Item> DynTrait<'borr,P,I>
where
    P: DerefMut,
    I: InterfaceConstsBound<'borr,Iterator = True,IteratorItem=Item>,
{
    pub fn skip_eager(&mut self, n: usize){
        let vtable=self.vtable();
        (vtable.iter().skip_eager)(self.as_abi_mut(),n);
    }

    pub fn extend_buffer(&mut self,buffer:&mut RVec<Item>,taking:ROption<usize>){
        let vtable=self.vtable();
        (vtable.iter().extend_buffer)(self.as_abi_mut(),buffer,taking);
    }
}


impl<'borr,P, I,Item> DoubleEndedIterator for DynTrait<'borr,P,I>
where
    Self:Iterator<Item=Item>,
    P: DerefMut,
    I: InterfaceConstsBound<'borr,DoubleEndedIterator = True,IteratorItem=Item>,
{

    fn next_back(&mut self)->Option<Item>{
        let vtable=self.vtable();
        (vtable.back_iter().next_back)(self.as_abi_mut()).into_rust()
    }
}


impl<'borr,P, I,Item> DynTrait<'borr,P,I>
where
    Self:Iterator<Item=Item>,
    P: DerefMut,
    I: InterfaceConstsBound<'borr,DoubleEndedIterator = True,IteratorItem=Item>,
{
    pub fn nth_back_(&mut self,nth:usize)->Option<Item>{
        let vtable=self.vtable();
        (vtable.back_iter().nth_back)(self.as_abi_mut(),nth).into_rust()
    }

    pub fn extend_buffer_back(&mut self,buffer:&mut RVec<Item>,taking:ROption<usize>){
        let vtable=self.vtable();
        (vtable.back_iter().extend_buffer_back)(self.as_abi_mut(),buffer,taking);
    }
}



unsafe impl<'borr,P,I> Send for DynTrait<'borr,P,I>
where
    P: Send,
    P: Deref,
    I: InterfaceConstsBound<'borr,Send = True>,
{}


unsafe impl<'borr,P,I> Sync for DynTrait<'borr,P,I>
where
    P: Sync,
    P: Deref,
    I: InterfaceConstsBound<'borr,Sync = True>,
{}


//////////////////////////////////////////////////////////////////

mod sealed {
    use super::*;
    pub trait Sealed {}
    impl<'borr,P,I> Sealed for DynTrait<'borr,P,I> 
    where I:InterfaceConstsBound<'borr>
    {}
}
use self::sealed::Sealed;

/// For accessing the Interface of a `DynTrait<Pointer<ZeroSized< Interface >>>`.
pub trait DynTraitBound<'borr>: Sealed {
    type Interface: InterfaceType;
}

impl<'borr,P, I> DynTraitBound<'borr> for DynTrait<'borr,P,I>
where
    P: Deref,
    I: InterfaceConstsBound<'borr>,
{
    type Interface = I;
}


/// For accessing the `Interface` in a `DynTrait<Pointer<ZeroSized< Interface >>>`.
pub type GetVWInterface<'borr,This>=
    <This as DynTraitBound<'borr>>::Interface;


//////////////////////////////////////////////////////////////////

/// Error for `DynTrait<_>` being unerased into the wrong type
/// with one of the `*unerased*` methods.
#[derive(Debug,Copy, Clone)]
pub struct UneraseError {
    expected_vtable_address: usize,
    expected_type_info:&'static TypeInfo,
    found_vtable_address: usize,
    found_type_info:&'static TypeInfo,
}


impl fmt::Display for UneraseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl ::std::error::Error for UneraseError {}
