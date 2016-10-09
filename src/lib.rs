#![feature(rustc_macro, rustc_macro_lib)]
#![cfg(not(test))]

extern crate rustc_macro;
extern crate syn;
#[macro_use]
extern crate quote;

use rustc_macro::TokenStream;
use syn::{Body, Field, Ident, MacroInput, VariantData};
use quote::{Tokens, ToTokens};

#[allow(dead_code)]
#[derive(Copy, Clone)]
enum BindStyle {
    Move,
    MoveMut,
    Ref,
    RefMut,
}

fn match_pattern<'a>(bind: BindStyle,
                     name: &Tokens,
                     vd: &'a VariantData)
                     -> (Tokens, Vec<BindingInfo<'a>>) {
    let mut t = Tokens::new();
    let mut matches = Vec::new();

    let prefix = match bind {
        BindStyle::Move => Tokens::new(),
        BindStyle::MoveMut => quote!(mut),
        BindStyle::Ref => quote!(ref),
        BindStyle::RefMut => quote!(ref mut),
    };

    name.to_tokens(&mut t);
    match *vd {
        VariantData::Unit => {}
        VariantData::Tuple(ref fields) => {
            t.append("(");
            for (i, field) in fields.iter().enumerate() {
                let ident: Ident = format!("__binding_{}", i).into();
                quote!(#prefix #ident ,).to_tokens(&mut t);
                matches.push(BindingInfo {
                    reference: quote!(#ident),
                    field: field,
                });
            }
            t.append(")");
        }
        VariantData::Struct(ref fields) => {
            t.append("{");
            for (i, field) in fields.iter().enumerate() {
                let field_name = field.ident.as_ref().unwrap();
                let ident: Ident = format!("__binding_{}", i).into();
                quote!(#field_name : #prefix #ident ,).to_tokens(&mut t);
                matches.push(BindingInfo {
                    reference: quote!(#ident),
                    field: field,
                });
            }
            t.append("}");
        }
    }

    (t, matches)
}

struct BindingInfo<'a> {
    pub reference: Tokens,
    pub field: &'a Field,
}

impl<'a> ToTokens for BindingInfo<'a> {
    fn to_tokens(&self, tokens: &mut Tokens) {
        self.reference.to_tokens(tokens);
    }
}

/// This method generates a match self {} body for the given macro input `input`
/// It should be used as follows:
///
/// ```
/// let body = combine_substructure(ast, BindStyle::Ref, |fields| {
///     let t = fields.iter().map(|&(ref i, _)| {
///         quote!(::some::Trait::Method(#i))
///     });
///     quote!(#(#t ;)*)
/// });
///
/// quote!(match *self { #body })
/// ```
fn combine_substructure<F>(input: &MacroInput, bind: BindStyle, func: F) -> Tokens
    where F: Fn(&[BindingInfo]) -> Tokens
{

    let ident = &input.ident;
    // Generate patterns for matching against all of the variants
    let variants = match input.body {
        Body::Enum(ref variants) => {
            variants.iter()
                .map(|variant| {
                    let variant_ident = &variant.ident;
                    match_pattern(bind, &quote!(#ident :: #variant_ident), &variant.data)
                })
                .collect()
        }
        Body::Struct(ref vd) => vec![match_pattern(bind, &quote!(#ident), vd)],
    };

    // Generate the final tokens
    let mut t = Tokens::new();
    // Call the passed in function with the passed in name/type pairs
    for (pat, bindings) in variants {
        let body = func(&bindings[..]);
        quote!(#pat => { #body }).to_tokens(&mut t);
    }

    t
}

fn entomb_body(ast: &MacroInput) -> Tokens {
    combine_substructure(ast, BindStyle::Ref, |fields| {
        let mut toks = Tokens::new();
        toks.append_all(fields.iter().map(|bind| {
            quote! {
                ::abomonation::Abomonation::entomb(#bind, _writer);
            }
        }));
        toks
    })
}

fn embalm_body(ast: &MacroInput) -> Tokens {
    combine_substructure(ast, BindStyle::RefMut, |fields| {
        let mut toks = Tokens::new();
        toks.append_all(fields.iter().map(|bind| {
            quote! {
                ::abomonation::Abomonation::embalm(#bind);
            }
        }));
        toks
    })
}

fn exhume_body(ast: &MacroInput) -> Tokens {
    combine_substructure(ast, BindStyle::RefMut, |fields| {
        let mut toks = Tokens::new();
        toks.append_all(fields.iter().map(|bind| {
            quote! {
                let temp = bytes;
                let exhume_result = ::abomonation::Abomonation::exhume(#bind, temp);
                bytes = if let Some(bytes) = exhume_result {
                    bytes
                } else {
                    return None
                };
            }
        }));
        toks
    })
}

#[rustc_macro_derive(Abomonation)]
pub fn derive_abomonation(input: TokenStream) -> TokenStream {
    let source = input.to_string();
    let ast = syn::parse_macro_input(&source).unwrap();

    // Get the name of the struct into a Tokens
    let name = &ast.ident;
    let (impl_generics, ty_generics, where_clause) = ast.generics.split_for_impl();

    // Generate the Entomb, Embalm, and Exhume function bodies
    let entomb = entomb_body(&ast);
    let embalm = embalm_body(&ast);
    let exhume = exhume_body(&ast);

    // Build the output tokens
    let result = quote! {
        // Original struct unmodified
        #ast

        impl #impl_generics ::abomonation::Abomonation for #name #ty_generics #where_clause {
            #[inline] unsafe fn entomb(&self, _writer: &mut Vec<u8>) {
                match *self { #entomb }
            }
            #[inline] unsafe fn embalm(&mut self) {
                match *self { #embalm }
            }
            #[inline] unsafe fn exhume<'a,'b>(&'a mut self, mut bytes: &'b mut [u8])
                                              -> Option<&'b mut [u8]> {
                match *self { #exhume }
                Some(bytes)
            }
        }
    };

    // Generate the final value as a TokenStream and return it
    result.to_string().parse().unwrap()
}
