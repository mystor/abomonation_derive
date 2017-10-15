extern crate syn;
#[macro_use]
extern crate synstructure;
#[macro_use]
extern crate quote;

decl_derive!([Abomonation] => derive_abomonation);

fn derive_abomonation(mut s: synstructure::Structure) -> quote::Tokens {
    let entomb = s.each(|bi| quote! {
        ::abomonation::Abomonation::entomb(#bi, _writer);
    });

    s.bind_with(|_| synstructure::BindStyle::RefMut);
    let embalm = s.each(|bi| quote! {
        ::abomonation::Abomonation::embalm(#bi);
    });

    let exhume = s.each(|bi| quote! {
        let temp = bytes;
        let exhume_result = ::abomonation::Abomonation::exhume(#bi, temp);
        bytes = if let Some(bytes) = exhume_result {
            bytes
        } else {
            return None
        };
    });

    s.bound_impl("::abomonation::Abomonation", quote! {
        #[inline] unsafe fn entomb(&self, _writer: &mut Vec<u8>) {
            match *self { #entomb }
        }
        #[inline] unsafe fn embalm(&mut self) {
            match *self { #embalm }
        }
        #[allow(unused_mut)]
        #[inline] unsafe fn exhume<'a,'b>(
            &'a mut self,
            mut bytes: &'b mut [u8]
        ) -> Option<&'b mut [u8]> {
            match *self { #exhume }
            Some(bytes)
        }
    })
}
