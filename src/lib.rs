#![recursion_limit="128"]

use quote::quote;
use std::collections::HashSet;
use synstructure::decl_derive;

decl_derive!([Abomonation, attributes(unsafe_abomonate_ignore)] => derive_abomonation);

fn derive_abomonation(mut s: synstructure::Structure) -> proc_macro2::TokenStream {
    s.filter(|bi| {
        !bi.ast().attrs.iter()
            .map(|attr| attr.parse_meta())
            .filter_map(Result::ok)
            .any(|attr| attr.path().is_ident("unsafe_abomonate_ignore"))
    });

    let entomb = s.each(|bi| quote! {
        ::abomonation::Entomb::entomb(#bi, _write)?;
    });

    // T::alignment() is the max of mem::align_of<T> and the U::alignment()s of
    // every U type used as a struct member or inside of an enum variant (which
    // includes the alignment of recursively abomonated data)
    //
    // Unfortunately, we cannot use synstructure's nice `fold()` convenience
    // here because it's based on generating match arms for a `self` value,
    // whereas here we're trying to implement an inherent type method without
    // having such a `self` handy.
    //
    // We can, however, use Structure::variants() and VariantInfo::bindings()
    // to enumerate all the types which _would appear_ in such match arms'
    // inner bindings.
    //
    let mut alignment = vec![
        quote!(let mut align = ::std::mem::align_of::<Self>();)
    ];
    let mut probed_types = HashSet::new();
    for variant_info in s.variants() {
        for binding_info in variant_info.bindings() {
            let binding_type = &binding_info.ast().ty;
            // Do not query a type's alignment() multiple times
            if probed_types.insert(binding_type) {
                alignment.push(
                    quote!(align = align.max(<#binding_type as ::abomonation::Entomb>::alignment());)
                );
            }
        }
    }
    alignment.push(quote!(align));

    s.bind_with(|_| synstructure::BindStyle::RefMut);

    let exhume = s.each(|bi| quote! {
        ::abomonation::Exhume::exhume(From::from(#bi), reader)?;
    });

    s.gen_impl(quote! {
        extern crate abomonation;
        extern crate std;

        gen unsafe impl abomonation::Entomb for @Self {
            unsafe fn entomb<W: ::std::io::Write>(
                &self,
                _write: &mut ::abomonation::align::AlignedWriter<W>
            ) -> ::std::io::Result<()> {
                match *self { #entomb }
                Ok(())
            }

            fn alignment() -> usize {
                #(#alignment)*
            }
        }

        gen unsafe impl<'de> abomonation::Exhume<'de> for @Self
            where Self: 'de,
        {
            #[allow(unused_mut)]
            unsafe fn exhume(
                self_: std::ptr::NonNull<Self>,
                reader: &mut ::abomonation::align::AlignedReader<'de>
            ) -> Option<&'de mut Self> {
                // FIXME: This (briefly) constructs an &mut _ to invalid data
                //        (via "ref mut"), which is UB. The proposed &raw mut
                //        operator would allow avoiding this.
                match *self_.as_ptr() { #exhume }
                Some(&mut *self_.as_ptr())
            }
        }
    })
}
