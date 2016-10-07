#![feature(rustc_macro, rustc_macro_lib)]
#![cfg(not(test))]

extern crate rustc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use rustc_macro::TokenStream;
use syn::{Body, Field, Ident};
use quote::Tokens;

/// Get the correct name for the field, given it's syn provided identifier, and
/// index in the field list.
fn field_name(ident: &Option<Ident>, index: usize) -> Ident {
    if let Some(ref id) = *ident {
        id.clone()
    } else {
        index.into()
    }
}

fn entomb_body(fields: &[Field]) -> Tokens {
    let mut toks = Tokens::new();
    toks.append_all(fields.iter().enumerate().map(|(i, field)| {
        let field_name = field_name(&field.ident, i);
        quote! {
            ::abomonation::Abomonation::entomb(&self.#field_name, _writer);
        }
    }));
    toks
}

fn embalm_body(fields: &[Field]) -> Tokens {
    let mut toks = Tokens::new();
    toks.append_all(fields.iter().enumerate().map(|(i, field)| {
        let field_name = field_name(&field.ident, i);
        quote! {
            ::abomonation::Abomonation::embalm(&mut self.#field_name);
        }
    }));
    toks
}

fn exhume_body(fields: &[Field]) -> Tokens {
    let mut toks = Tokens::new();
    toks.append_all(fields.iter().enumerate().map(|(i, field)| {
        let field_name = field_name(&field.ident, i);
        quote! {
            let temp = bytes;
            let exhume_result = ::abomonation::Abomonation::exhume(&mut self.#field_name, temp);
            bytes = if let Some(bytes) = exhume_result {
                bytes
            } else {
                return None
            };
        }
    }));
    toks
}

#[rustc_macro_derive(Abomonation)]
pub fn derive_abomonation(input: TokenStream) -> TokenStream {
    let source = input.to_string();
    let ast = syn::parse_macro_input(&source).unwrap();

    // Get the name of the struct into a Tokens
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // Extract the fields from the parsed struct declaration
    let fields = match ast.body {
        Body::Struct(ref variant_data) => variant_data.fields(),
        Body::Enum(_) => panic!("Abomonation doesn't support Enums"),
    };

    // Generate the Entomb, Embalm, and Exhume function bodies
    let entomb = entomb_body(fields);
    let embalm = embalm_body(fields);
    let exhume = exhume_body(fields);

    // Build the output tokens
    let result = quote! {
        // Original struct unmodified
        #ast

        impl #impl_generics ::abomonation::Abomonation for #name #ty_generics #where_clause {
            #[inline] unsafe fn entomb(&self, _writer: &mut Vec<u8>) {
                #entomb
            }
            #[inline] unsafe fn embalm(&mut self) {
                #embalm
            }
            #[inline] unsafe fn exhume<'a,'b>(&'a mut self, mut bytes: &'b mut [u8])
                                              -> Option<&'b mut [u8]> {
                #exhume
                Some(bytes)
            }
        }
    };

    // Generate the final value as a TokenStream and return it
    result.to_string().parse().unwrap()
}
