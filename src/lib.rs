#![feature(rustc_macro, rustc_macro_lib)]

extern crate rustc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use rustc_macro::TokenStream;
use syn::{VariantData, Body, Field, Ident};
use quote::Tokens;

fn raw_token(s: &str) -> Tokens {
    let mut t = Tokens::new();
    t.append(s);
    t
}

/// Get the correct name for the field, given it's syn provided identifier, and
/// index in the field list.
fn field_name(ident: &Option<Ident>, index: usize) -> Tokens {
    if let Some(ref id) = *ident {
        raw_token(&id.to_string())
    } else {
        raw_token(&format!("{}", index))
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
    let mut name = raw_token(&ast.ident.to_string());

    // Extract the fields from the parsed struct declaration
    let fields: &[Field] = match ast.body {
        Body::Struct(VariantData::Struct(ref fields)) |
        Body::Struct(VariantData::Tuple(ref fields)) => fields,
        Body::Struct(VariantData::Unit) => &[],
        Body::Enum(_) => panic!("Abomonation doesn't support Enums"),
    };

    // Generate the Entomb, Embalm, and Exhume function bodies
    let entomb = entomb_body(fields);
    let embalm = embalm_body(fields);
    let exhume = exhume_body(fields);

    // Build the output tokens
    let result = quote! {
        impl ::abomonation::Abomonation for #name {
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
    format!("{}\n{}", source, result.to_string()).parse().unwrap()
}
