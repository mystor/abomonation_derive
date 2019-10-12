#![recursion_limit="128"]

use synstructure::decl_derive;
use quote::quote;

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

    let extent = s.each(|bi| quote! {
        sum += ::abomonation::Entomb::extent(#bi);
    });

    s.bind_with(|_| synstructure::BindStyle::RefMut);

    let exhume = s.each(|bi| quote! {
        bytes = ::abomonation::Exhume::exhume(From::from(#bi), bytes)?;
    });

    s.gen_impl(quote! {
        extern crate abomonation;
        extern crate std;

        gen unsafe impl abomonation::Entomb for @Self {
            unsafe fn entomb<W: ::std::io::Write>(&self, _write: &mut W) -> ::std::io::Result<()> {
                match *self { #entomb }
                Ok(())
            }

            #[allow(unused_mut)]
            fn extent(&self) -> usize {
                let mut sum = 0;
                match *self { #extent }
                sum
            }
        }

        gen unsafe impl<'de> abomonation::Exhume<'de> for @Self
            where Self: 'de,
        {
            #[allow(unused_mut)]
            unsafe fn exhume(
                self_: std::ptr::NonNull<Self>,
                mut bytes: &'de mut [u8]
            ) -> Option<&'de mut [u8]> {
                // FIXME: This (briefly) constructs an &mut _ to invalid data
                //        (via "ref mut"), which is UB. The proposed &raw mut
                //        operator would allow avoiding this.
                match *self_.as_ptr() { #exhume }
                Some(bytes)
            }
        }
    })
}
