/*!

Contains `DynTrait<_>`'s vtable,and related types/traits.

*/
use std::{
    fmt::{self, Debug},
    marker::PhantomData,
};

use super::{
    *,
    c_functions::*,
    iterator::{IteratorFns,MakeIteratorFns},
    traits::{IteratorItemOrDefault},
};

use crate::{
    StableAbi,
    abi_stability::{Tag,SharedStableAbi},
    marker_type::ErasedObject,
    prefix_type::{PrefixTypeTrait,WithMetadata,panic_on_missing_fieldname},
    std_types::{Tuple3,RSome,RNone},
};

use core_extensions::{ResultLike, StringExt,TypeIdentity};





#[doc(hidden)]
/// Returns the vtable used by DynTrait to do dynamic dispatch.
pub trait GetVtable<'borr,This,ErasedPtr,OrigPtr,I:InterfaceConstsBound<'borr>> {
    
    const TMP_VTABLE:VTableVal<'borr,ErasedPtr,I>;

    const GET_VTABLE:*const WithMetadata<VTableVal<'borr,ErasedPtr,I>>=
        &WithMetadata::new(
            PrefixTypeTrait::METADATA,
            Self::TMP_VTABLE
        );


    /// Retrieves the VTable of the type.
    fn get_vtable<'a>() -> &'a VTable<'borr,ErasedPtr,I>
    where
        This: 'a,
    {
        // I am just getting a vtable
        unsafe { (*Self::GET_VTABLE).as_prefix() }
    }
}



macro_rules! declare_meta_vtable {
    (
        interface=$interf:ident;
        value=$value:ident;
        erased_pointer=$erased_ptr:ident;
        original_pointer=$orig_ptr:ident;

        marker_traits[
            $([
                impl $marker_trait:ident where [ $($phantom_where_clause:tt)* ]
            ])*
        ]

        $([
            $( #[$field_attr:meta] )*
            $field:ident : $field_ty:ty ;
            priv $priv_field:ident;
            option=$option_ty:ident,$some_constr:ident,$none_constr:ident;

            $(struct_bound=$struct_bound:expr;)*
            
            impl[$($impl_params:tt)*] VtableFieldValue<$selector:ident>
            where [ $($where_clause:tt)* ]
            { $field_value:expr }
        ])*
    ) => (

        /// This is the vtable for DynTrait<_>,
        ///
        #[repr(C)]
        #[derive(StableAbi)]
        #[sabi(
            // debug_print,
            inside_abi_stable_crate,
            kind(Prefix(prefix_struct="VTable")),
            missing_field(panic),
            prefix_bound="I:InterfaceConstsBound<'borr>",
            bound="<I as SharedStableAbi>::StaticEquivalent:InterfaceBound<'static>",
            bound="<I as InterfaceBound<'borr>>::IteratorItem:StableAbi",
            $($(bound=$struct_bound,)*)*
        )]
        pub struct VTableVal<'borr,$erased_ptr,$interf>
        where $interf:InterfaceBound<'borr>
        {
            pub type_info:&'static TypeInfo,
            _marker:PhantomData<extern fn()->Tuple3<$erased_ptr,$interf,&'borr()>>,
            pub drop_ptr:unsafe extern "C" fn(&mut $erased_ptr),
            $(
                $( #[$field_attr] )*
                $priv_field:$option_ty<($field_ty)>,
            )*
        }


        impl<'borr,$erased_ptr,$interf> VTable<'borr,$erased_ptr,$interf>
        where   
            $interf:InterfaceConstsBound<'borr>,
        {
            $(
                pub fn $field(&self)->($field_ty)
                where
                    $interf:InterfaceType<$selector=True>,
                {
                    const NAME:&'static &'static str=&stringify!($field);

                    match self.$priv_field().into() {
                        Some(v)=>v,
                        None=>panic_on_missing_fieldname::<
                            VTableVal<$erased_ptr,$interf>,
                            $field_ty
                        >(
                            NAME,
                            self._prefix_type_layout(),
                        )
                    }
                }
            )*
        }

        /// Returns the type of a vtable field.
        pub type VTableFieldType<'borr,Selector,$value,$erased_ptr,$orig_ptr,$interf>=
            <Selector as VTableFieldType_<'borr,$value,$erased_ptr,$orig_ptr,$interf>>::Field;

        /// Returns the type of a vtable field.
        pub trait VTableFieldType_<'borr,$value,$erased_ptr,$orig_ptr,$interf>{
            type Field;
        }

        /// Returns the value of a vtable field in the current binary
        /// (this can be a different value in a dynamically_linked_library/executable).
        pub trait VTableFieldValue<'borr,Ty,IsImpld,$value,$erased_ptr,$orig_ptr,$interf>{
            const FIELD:Ty;
        }

        pub trait MarkerTrait<'borr,IsImpld,$value,$erased_ptr,$orig_ptr>{}


        $(
            impl<'borr,$value,$erased_ptr,$orig_ptr,$interf> 
                VTableFieldType_<'borr,$value,$erased_ptr,$orig_ptr,$interf> 
            for trait_selector::$selector 
            where 
                $interf:InterfaceBound<'borr>,
            {
                type Field=$field_ty;
            }

            
            impl<'borr,AnyFieldTy,$value,$erased_ptr,$orig_ptr,$interf>
                VTableFieldValue<
                    'borr,$option_ty<AnyFieldTy>,False,$value,$erased_ptr,$orig_ptr,$interf
                >
            for trait_selector::$selector
            {
                const FIELD:$option_ty<AnyFieldTy>=$none_constr;
            }

            impl<'borr,FieldTy,$value,$erased_ptr,$orig_ptr,$interf,$($impl_params)*>
                VTableFieldValue<
                    'borr,$option_ty<FieldTy>,True,$value,$erased_ptr,$orig_ptr,$interf
                >
            for trait_selector::$selector
            where 
                $interf:InterfaceBound<'borr>,
                $field_ty:TypeIdentity<Type=FieldTy>,
                FieldTy:Copy,
                $($where_clause)*
            {
                const FIELD:$option_ty<FieldTy>=
                    $some_constr(type_identity!($field_ty=>FieldTy;$field_value));
            }
        )*



        impl<'borr,Anything,$value,$erased_ptr,$orig_ptr> 
            MarkerTrait<'borr,False,$value,$erased_ptr,$orig_ptr> 
        for Anything
        {}

        $(
            impl<'borr,$value,$erased_ptr,$orig_ptr> 
                MarkerTrait<'borr,True,$value,$erased_ptr,$orig_ptr> 
            for trait_selector::$marker_trait
            where $($phantom_where_clause)*
            {}
        )*

        ///////////////////////////////////////////////////////////

        /// Contains marker types representing traits of the same name.
        pub mod trait_selector{
            $(
                /// Marker type representing the trait of the same name.
                pub struct $marker_trait;
            )*
            $(
                /// Marker type representing the trait of the same name.
                pub struct $selector;
            )*
        }


        impl<'borr,This,$value,$erased_ptr,$orig_ptr,$interf> 
            GetVtable<'borr,$value,$erased_ptr,$orig_ptr,$interf>
        for This
        where
            This:ImplType<Interface=$interf>,
            $interf:InterfaceConstsBound<'borr>,
            $(
                trait_selector::$marker_trait:
                    MarkerTrait<'borr,$interf::$marker_trait,$value,$erased_ptr,$orig_ptr>,
            )*
            $(
                trait_selector::$selector:VTableFieldValue<
                    'borr,
                    $option_ty<$field_ty>,
                    $interf::$selector,
                    $value,
                    $erased_ptr,
                    $orig_ptr,
                    $interf,
                >,
            )*
        {
            const TMP_VTABLE:VTableVal<'borr,$erased_ptr,$interf>=VTableVal{
                type_info:This::INFO,
                drop_ptr:drop_pointer_impl::<$orig_ptr,$erased_ptr>,
                $(
                    $priv_field:
                        <trait_selector::$selector as
                            VTableFieldValue<
                                $option_ty<VTableFieldType<
                                    trait_selector::$selector,
                                    $value,
                                    $erased_ptr,
                                    $orig_ptr,
                                    $interf,
                                >>,
                                $interf::$selector,
                                $value,
                                $erased_ptr,
                                $orig_ptr,
                                $interf,
                            >
                        >::FIELD,
                )*
                _marker:PhantomData,
            };

        }


        /// Trait used to capture all the bounds that an InterfaceType 
        /// when used as a type parameter of a type.
        #[allow(non_upper_case_globals)]
        pub trait InterfaceBound<'borr>:InterfaceType {
            #[doc(hidden)]
            const __InterfaceBound_BLANKET_IMPL:PrivStruct<Self>;

            type IteratorItem:'borr;
        }   

        /// Associated constants derived from an InterfaceType.
        #[allow(non_upper_case_globals)]
        pub trait InterfaceConstsBound<'borr>:InterfaceBound<'borr> {
            const TAG:Tag;

            $( const $selector:bool; )*

            #[doc(hidden)]
            const __InterfaceConstsBound_BLANKET_IMPL:PrivStruct<Self>;
        }   


        #[allow(non_upper_case_globals)]
        impl<'borr,I> InterfaceBound<'borr> for I
        where 
            I:InterfaceType,
            I:IteratorItemOrDefault<'borr,<I as InterfaceType>::Iterator>,
        {
            const __InterfaceBound_BLANKET_IMPL:PrivStruct<Self>=
                PrivStruct(PhantomData);

            type IteratorItem=
                <I as IteratorItemOrDefault<'borr,<I as InterfaceType>::Iterator>>::Item ;
        }


        #[allow(non_upper_case_globals)]
        impl<'borr,I> InterfaceConstsBound<'borr> for I
        where 
            I:InterfaceBound<'borr>,
            $( I::$marker_trait:Boolean, )*
            $( I::$selector:Boolean, )*
        {
            const TAG:Tag={
                const fn str_if(cond:bool,s:&'static str)->Tag{
                    [ Tag::null(), Tag::str(s) ][cond as usize]
                }

                tag!{{
                    "auto traits"=>tag![[
                        $(
                            str_if(
                                <I::$marker_trait as Boolean>::VALUE,
                                stringify!($marker_trait)
                            ),
                        )*
                    ]],
                    "required traits"=>tag!{{
                        $(
                            str_if(
                                <I::$selector as Boolean>::VALUE,
                                stringify!($selector)
                            ),
                        )*
                    }}
                }}
            };

            $( 
                const $selector:bool=<I::$selector as Boolean>::VALUE;
            )*
            
            const __InterfaceConstsBound_BLANKET_IMPL:PrivStruct<Self>=
                PrivStruct(PhantomData);
        }


        impl<'borr,$erased_ptr,$interf> Debug for VTable<'borr,$erased_ptr,$interf> 
        where
            $interf:InterfaceConstsBound<'borr>,
        {
            fn fmt(&self,f:&mut fmt::Formatter<'_>)->fmt::Result {
                f.debug_struct("VTable")
                    .field("type_info",&self.type_info())
                    // $(
                    //     .field(
                    //         stringify!($field),
                    //         &format_args!("{:x}",self.$priv_field().map_or(0,|x|x as usize))
                    //     )
                    // )*
                    .finish()
            }
        }

    )
}

declare_meta_vtable! {
    interface=I;
    value  =T;
    erased_pointer=ErasedPtr;
    original_pointer=OrigP;

    marker_traits[
        [
            impl Send where [OrigP:Send]
        ]
        [
            impl Sync where [OrigP:Sync]
        ]
    ]

    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::Clone")]
        clone_ptr:    extern "C" fn(&ErasedPtr)->ErasedPtr;
        priv _clone_ptr;
        option=Option,Some,None;
        
        impl[] VtableFieldValue<Clone>
        where [OrigP:Clone]
        {
            clone_pointer_impl::<OrigP,ErasedPtr>
        }
    ]
    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::Default")]
        default_ptr: extern "C" fn()->ErasedPtr ;
        priv _default_ptr;
        option=Option,Some,None;
        impl[] VtableFieldValue<Default>
        where [OrigP:Default]
        {
            default_pointer_impl::<OrigP,ErasedPtr>
        }
    ]
    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::Display")]
        display:    extern "C" fn(&ErasedObject,FormattingMode,&mut RString)->RResult<(),()>;
        priv _display;
        option=Option,Some,None;
        impl[] VtableFieldValue<Display>
        where [T:Display]
        {
            display_impl::<T>
        }
    ]
    [
    #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::Debug")]
        debug:      extern "C" fn(&ErasedObject,FormattingMode,&mut RString)->RResult<(),()>;
        priv _debug;
        option=Option,Some,None;
        impl[] VtableFieldValue<Debug>
        where [T:Debug]
        {
            debug_impl::<T>
        }
    ]
    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::Serialize")]
        serialize:  extern "C" fn(&ErasedObject)->RResult<RCow<'_,RStr<'_>>,RBoxError>;
        priv _serialize;
        option=Option,Some,None;
        impl[] VtableFieldValue<Serialize>
        where [ T:SerializeImplType, ]{
            serialize_impl::<T>
        }
    ]
    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::PartialEq")]
        partial_eq: extern "C" fn(&ErasedObject,&ErasedObject)->bool;
        priv _partial_eq;
        option=Option,Some,None;
        impl[] VtableFieldValue<PartialEq>
        where [T:PartialEq,]
        {
            partial_eq_impl::<T>
        }
    ]
    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::Ord")]
        cmp:        extern "C" fn(&ErasedObject,&ErasedObject)->RCmpOrdering;
        priv _cmp;
        option=Option,Some,None;
        impl[] VtableFieldValue<Ord>
        where [T:Ord,]
        {
            cmp_ord::<T>
        }
    ]
    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::PartialOrd")]
        partial_cmp:extern "C" fn(&ErasedObject,&ErasedObject)->ROption<RCmpOrdering>;
        priv _partial_cmp;
        option=Option,Some,None;
        impl[] VtableFieldValue<PartialOrd>
        where [T:PartialOrd,]
        {
            partial_cmp_ord::<T>
        }
    ]
    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::Hash")]
        #[sabi(last_prefix_field)]
        hash:extern "C" fn(&ErasedObject,trait_objects::HasherObject<'_>);
        priv _hash;
        option=Option,Some,None;
        impl[] VtableFieldValue<Hash>
        where [T:Hash]
        {
            hash_Hash::<T>
        }
    ]
    [
        #[sabi(accessible_if="<I as InterfaceConstsBound<'borr>>::Iterator")]
        #[sabi(last_prefix_field)]
        iter:IteratorFns< <I as InterfaceBound<'borr>>::IteratorItem >;
        priv _iter;
        option=ROption,RSome,RNone;
        impl[] VtableFieldValue<Iterator>
        where [
            T:Iterator,
            T::Item:'borr,
            I:InterfaceBound<'borr,IteratorItem=<T as Iterator>::Item>,
        ]{
            MakeIteratorFns::<T>::NEW
        }
    ]
}

//////////////


/// Used to prevent InterfaceBound being implemented outside this module,
/// since it is only constructed in the impl of InterfaceBound in this module.
#[doc(hidden)]
pub struct PrivStruct<T>(PhantomData<T>);

