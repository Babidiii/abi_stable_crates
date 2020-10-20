/*!

This attribute generates an ffi-safe trait object on the trait it's applied to.

All items outside the list of generated items comes from `abi_stable::sabi_trait`.


# Supertraits.

By default these are the supertraits that `#[sabi_trait]` traits can have:

- lifetimes:It can be a lifetime declared by the trait,or `'static`.

- `Debug`

- `Display`

- `std::error::Error`: Written as `Error`: The `Error` methods aren't delegated to,
it uses the default implementation,

- `Clone`

- `Send`

- `Sync`

To be able to have more supertraits you must use the `#[sabi(use_dyntrait)]` helper attribute,
which changes the underlying implementation from `RObject<_>` to `DynTrait<_>`,
allowing these supertraits:

- `Iterator`: requires the Item type to be specified.

- `DoubleEndedIterator`: requires the Item type to be specified.

- `std::fmt::Write`: Written as `FmtWrite`

- `std::io::Write`: Written as `IoWrite`

- `std::io::Seek`: Written as `IoSeek`

- `std::io::Read`: Written as `IoRead`

- `std::io::BufRead`: Written as `IoBufRead`

- `Eq`

- `PartialEq`

- `Ord`

- `PartialOrd`

- `Hash`



### Supertrait Extensibility

The properties described below are checked when `abi_stable` loads a dynamic library.

Traits can add non-marker supertraits in minor versions without breaking ABI compatibility,
with the (non-ABI related) caveats described in the extensibility section.

Traits cannot add marker supertraits in minor versions ABI compatibly,
because that would cause problems with thread/memory safety if allowed.
If it were allowed,`!Send` trait objects could be passed from a binary to 
a dynamic library(where the trait object type is `Send`),
and that would be Undefined Behavior in many situations.

# Extensibility

`#[sabi_trait]` trait objects are (ABI-wise) safe to extend in minor versions,
so long as methods are always added at the end,preferably as default methods.

A library will not load (through safe means) if methods are added anywhere but the end.

Accidentally calling newer methods on trait objects from older versions of a
library will cause a panic at runtime,unless it has a default implementation
(within the trait definition that `#[sabi_trait]` can see).

Panics can only happen if one loads multiple versions of a library,
where the trait is extended in each version(without using default methods),
and passes trait objects among those libraries.

# Generated items.

This is a nonexhaustive list of the items generated by the attribute,
where `Trait` is the name of the annotated trait.

### `Trait_trait`

This is the module inside of which all the items are generated.

These are the items reexported from the module:

- `Trait`:The trait itself.

- `Trait_TO`:The trait object for the trait.

- `Trait_MV`:
    A helper type used to construct the vtable for the trait object in constants.

- `Trait_CTO`:A type alias for the trait object which is constructible in constants.


###  Trait_TO 

The ffi-safe trait object.

<br>

This only implements `Trait` if all the methods are callable,
when the wrapped pointer type implements traits for these methods:

- `&self` method: requires `Deref<Target=()>`.
- `&mut self` method: requires `DerefMut<Target=()>`.
- `self` method: requires `OwnedPointer<Target=()>`.

<br>

`Trait_TO` also has inherent method equivalents of the trait methods,
only requiring the pointer to implement the trait in the individual methods
(instead of putting those bounds in the impl block itself).

<br>

Trait_TO has these generic parameters(in order):

- `'trait_lifetime_n`: The lifetime parameters of the trait,if any.

- `'lt`:
This is the lifetime of the type that the trait object was constructed with.
If the trait requires `'static`(in the list of supertraits),
then it doesn't have this lifetime parameter.

- `Pointer`: 
    An pointer whose referent has been erased,
    most commonly `RBox<()>`/`RArc<()>`/`&()`/`&mut ()`.

- `trait_type_param_n`: The type parameters of the trait.

- `trait_const_param_n`: The const parameters of the trait.

- `trait_assoc_type_n`: The associated types of the trait.


A trait defined like this:`trait Foo<'a,T,U>{ type Hello; type World; }`,
has this trait object:`Foo_TO<'a,'lt,Pointer,T,U,Hello,World>`.

<br>

One can access the underlying implementation of the trait object through the `obj` field,
allowing one to call these methods(a nonexhaustive list):

- into_unerased_impltype(only DynTrait)

- as_unerased_impltype(only DynTrait)

- as_unerased_mut_impltype(only DynTrait)

- into_unerased

- as_unerased

- as_unerased_mut

To reconstruct `Trait_TO` from its underlying implementation,
you can use the `Trait_TO::from_sabi` associated function.

###  Trait_CTO

A type alias for the type of the trait objct that is constructible in constants,
with the `from_const` constructor function.

Constructed with `Trait_CTO::from_const(&value,Trait_MV::VTABLE)`.

Trait_CTO has these generic parameters(in order):

- `'trait_lifetime_n`: The lifetime parameters of the trait,if any.

- `'lt`:this is the lifetime of the type that the trait object was construct with.
If the trait requires `'static`(in the list of supertraits),
then it doesn't have this lifetime parameter.

- `'_ref`:this is the lifetime of the reference that this was constructed with.

- `trait_type_param_n`: The type parameters of the trait.

- `trait_const_param_n`: The const parameters of the trait.

- `trait_assoc_type_n`: The associated types of the trait.


Example: `Trait_CTO<'lt, 'r, u8, u64, 10, AssocFoo>`

###  Trait_MV

A helper type used to construct the vtable of the trait object.

###  Trait_TO::from_ptr 

A constructor for the trait object,which takes a pointer to a value that implements the trait.

Generally it is called like this
`Trait_TO::from_ptr( pointer,<Unerasability> )`.<br>
or like this(if you don't want to write the type of the returned trait object):<br>
`Trait_TO::<_,TraitParam0, TraitParam1>::from_ptr( pointer,<Unerasability> )`.

Where `<Unerasability>` can be either:

-`TU_Unerasable`:
    Which allows the trait object to be unerased,requires that the value implements any.

.`TU_Opaque`:Which does not allow the trait object to be unerased.

Where `TraitParam*` are the type parameters of the trait.

###  Trait_TO::from_value 

A constructor for the trait object,which takes a value that implements the trait.

This is equivalent to calling `Trait_TO::from_ptr` with `RBox::new(value)`.

###  Trait_TO::from_sabi 

Constructs the trait object from its underlying implementation,
either `RObject` or `DynTrait` depending on whether the
`#[sabi(use_dyntrait)]` helper attribute was used.

###  Trait_TO::from_const

Constructs the trait object from a reference to a constant value,
eg:`Trait_CTO::from_const(&value,<Unerasability>,Trait_MV::VTABLE)`.

Where `<Unerasability>` can be either:

-`TU_Unerasable`:
    Which allows the trait object to be unerased,requires that the value implements any.

.`TU_Opaque`:Which does not allow the trait object to be unerased.


### Trait_TO::sabi_reborrow

Reborrows the trait object,going from `&Trait_TO<'lt,SomePtr<()>>` to `Trait_TO<'lt,&()>`.

This is only generated if the trait has both or neither Send and Sync as supertraits.

### Trait_TO::sabi_reborrow_mut

Reborrows the trait object mutably,
going from `&mut Trait_TO<'lt,SomePtr<()>>` to `Trait_TO<'lt,&mut ()>`.

This is only generated if the trait has both or neither Send and Sync as supertraits.

###  Trait 

The trait is defined similarly to how it is before being transformed by the 
`#[sabi_trait]` attribute.

These are the differences:

- If there is a by-value method,a `Self:Sized` constraint will be added automatically.

- Lifetime supertraits are stripped,because they disallow the trait object to be 
constructed with a reference of a smaller lifetime.

### Trait_Bounds

A trait used as an alias for `Trait + lifetime supertraits`,
because lifetime supertraits are stripped from Trait.

# VTable attributes

To pass attributes to the generated vtable you can use the `#[sabi(  )]` attributes 
that are valid for `#[derive(StableAbi)]`.

[Here is the documentation for the derive macro.
](../stable_abi_derive/index.html)

# Trait attributes.

These are attributes for the generated trait,applied on the trait(not on methods).

###  `#[sabi(no_trait_impl)]`

Disables the implementation of the trait for the trait object,
you can still call the inherent versions of those methods on the trait object.

This is useful to reduce compile-time overhead,
and to allow users to declare a blanket(generic) implementation of the trait.

###  `#[sabi(no_default_fallback)]`

Stops using default implementation of methods (from the trait declaration) 
as the fallback implementation of the method when it's not in the vtable,
because the trait object comes from a previous version of the library.

By using this attribute,defaulted methods will behave the same as 
non-defaulted methods when they don't exist in the vtable.

### `#[sabi(debug_print_trait)]`

Prints the output generated by the attribute macro,

Note that this does not expand the output of the 
`#[derive(StableAbi)]` attribute on the vtable.

### `#[sabi(use_dyntrait)]`

Changes how the trait object is implemented to use `DynTrait` instead of `RObject`,
this allows using more traits,with the (potential) cost of having more overhead.

# Associated types

The only valid way to refer to associated types in the trait declaration is with 
`Self::AssocType` syntax.

Associated types in the trait object are transformed into type parameters 
that come after those of the trait.

# Object safety

Trait objects generated using this attribute have similar restrictions to built-in trait objects:

- `Self` can only be used to access associated types 
    (using the `Self::AssocType` syntax).

- `self` is a valid method receiver,
    this requires that the pointer that the generated trait object wraps 
    implements `abi_stable::pointer_trait::OwnedPointer`.

# Questions and Answers

**Question:** Why does Calling from_ptr/from_value give me a expected a `'static` value error?

Answer: There are 3 possible reasons

- 1: Because the trait has a `'static` supertrait bound.

- 2: Because the trait has one of the comparison traits
(`Eq`/`PartialEq`/`Ord`/`PartialOrd`)
as supertraits.
This requires the type to be `'static` because comparing trait objects requires 
constructing a `std::any::TypeId`,which itself requires `'static` to be constructed.

- 3: Because you passed `TU_Unerasable` to the constructor function,
which requires constructing a `std::any::TypeId`
(to unerase the trait object back into the value),
which itself requires `'static` to be constructed.

# Examples

###  Dictionary trait 

```rust
use abi_stable::{
    StableAbi,
    sabi_trait,
    sabi_trait::prelude::*,
    std_types::{RBox,RArc,RString,RStr,ROption,RNone},
};

use std::{
    collections::HashMap,
    fmt::Debug,
};

#[sabi_trait]
pub trait Dictionary:Debug+Clone{
    type Value;

    fn get(&self,key:RStr<'_>)->Option<&Self::Value>;

    /// The `#[sabi(last_prefix_field)]` attribute here means that this is the last method 
    /// that was defined in the first compatible version of the library
    /// (0.1.0, 0.2.0, 0.3.0, 1.0.0, 2.0.0 ,etc),
    /// requiring new methods to always be added below preexisting ones.
    /// 
    /// The `#[sabi(last_prefix_field)]` attribute would stay on this method until the library 
    /// bumps its "major" version,
    /// at which point it would be moved to the last method at the time.
    /// 
    #[sabi(last_prefix_field)]
    fn insert(&mut self,key:RString,value:Self::Value)->ROption<Self::Value>;

    /// You can add defaulted methods in minor versions(it's not a breaking change).
    fn contains(&self,key:RStr<'_>)->bool{
        self.get(key).is_some()
    }
}


# fn main() {

{
    impl<V> Dictionary for HashMap<RString,V>
    where
        V:Debug+Clone
    {
        type Value=V;
        fn get(&self,key:RStr<'_>)->Option<&V>{
            self.get(key.as_str())
        }
        fn insert(&mut self,key:RString,value:V)->ROption<V>{
            self.insert(key,value)
                .into()
        }
    }

    let mut map=HashMap::<RString,u32>::new();
    map.insert("hello".into(),100);
    map.insert("world".into(),10);

    {
        // This type annotation is for the reader
        //
        // You can unerase trait objects constructed with `TU_Unerasable` 
        // (as opposed to `TU_Opaque`,which can't be unerased).
        let mut object:Dictionary_TO<'_,RBox<()>,u32>=
            Dictionary_TO::from_value(map.clone(),TU_Unerasable);

        assert_eq!(Dictionary::get(&object,"hello".into()),Some(&100));
        assert_eq!(object.get("hello".into()),Some(&100)); // Inherent method call
        
        assert_eq!(Dictionary::get(&object,"world".into()),Some(&10));
        assert_eq!(object.get("world".into()),Some(&10));  // Inherent method call

        object.insert("what".into(),99); // Inherent method call

        // You can only unerase a trait object if it was constructed with `TU_Unerasable`
        // and it's being unerased into a type that implements `std::any::Any`.
        let map:RBox<HashMap<RString,u32>>=object.obj.into_unerased().unwrap();

        assert_eq!(map.get("hello".into()), Some(&100));
        assert_eq!(map.get("world".into()), Some(&10));
        assert_eq!(map.get("what".into()), Some(&99));
    }
    {
        let arc=RArc::new(map.clone());
        // This type annotation is for the reader
        //
        // You can unerase trait objects constructed with `TU_Unerasable` 
        // (as opposed to `TU_Opaque`,which can't be unerased).
        let object:Dictionary_TO<'_,RArc<()>,u32>=
            Dictionary_TO::from_ptr(arc,TU_Unerasable);

        assert_eq!(object.get("world".into()),Some(&10));
        
        // Can't call these methods on `Dictionary_TO<RArc<()>,..>`
        // because `RArc<_>` doesn't implement DerefMut.
        //
        // assert_eq!(Dictionary::get(&object,"hello"),Some(&100));
        //
        // object.insert("what".into(),99);
        // Dictionary::insert(&mut object,"what".into(),99);
        

        let map:RArc<HashMap<RString,u32>>=object.obj.into_unerased().unwrap();
        assert_eq!(map.get("hello".into()), Some(&100));
        assert_eq!(map.get("world".into()), Some(&10));
    }

}

{
    impl Dictionary for (){
        type Value=RString;
        fn get(&self,_:RStr<'_>)->Option<&RString>{
            None
        }
        fn insert(&mut self,_:RString,_:RString)->ROption<RString>{
            RNone
        }
    }

    // This type annotation is for the reader
    let object:Dictionary_TO<'_,RBox<()>,RString>=
        Dictionary_TO::from_value( () ,TU_Opaque);

    assert_eq!(object.get("hello".into()),None);
    assert_eq!(object.get("world".into()),None);

    // Cannot unerase trait objects created with `TU_Opaque`.
    assert_eq!(object.obj.into_unerased::<()>().ok(),None);
}

# }

```


# Constructing a trait object in a constant

This shows how one can construct a `#[sabi_trait]` generated trait object in a constant/static.

```rust

use abi_stable::{
    sabi_trait::TU_Opaque,
    sabi_trait,

};

#[sabi_trait]
pub trait StaticSet:Sync+Send+Debug+Clone{
    type Element;

    /// Whether the set contains the key.
    fn contains(&self,key:&Self::Element)->bool;
}

impl<'a,T> StaticSet for &'a [T]
where
    T:std::fmt::Debug+Sync+Send+std::cmp::PartialEq
{
    type Element=T;
    
    fn contains(&self,key:&Self::Element)->bool{
        (**self).contains(key)
    }
}

const CARDS:&'static [char]=&['A','2','3','4','5','6','7','8','9','J','Q','K'];

static IS_CARD:StaticSet_CTO<'static,'static,char>=
    StaticSet_CTO::from_const(
        &CARDS,
        TU_Opaque,
        StaticSet_MV::VTABLE,
    );

# fn main(){

assert!( IS_CARD.contains(&'A') );
assert!( IS_CARD.contains(&'4') );
assert!( IS_CARD.contains(&'7') );
assert!( IS_CARD.contains(&'9') );
assert!( IS_CARD.contains(&'J') );

assert!( ! IS_CARD.contains(&'0') );
assert!( ! IS_CARD.contains(&'1') );
assert!( ! IS_CARD.contains(&'B') );

# }

```




*/
