#![recursion_limit = "128"]

mod utils;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use syn::{
    parse_quote, Attribute, DataEnum, DataStruct, DeriveInput, FieldsNamed, FieldsUnnamed,
    GenericParam, Generics,
};
use utils::{bounds, get_style, omit_bounds, skip, with, Style};

#[derive(Debug, Clone, Default)]
struct MethodData {
    entomb: proc_macro2::TokenStream,
    exhume: proc_macro2::TokenStream,
    extent: proc_macro2::TokenStream,
}

#[proc_macro_derive(
    Abomonation,
    attributes(
        abomonation_omit_bounds,
        abomonation_bounds,
        abomonate_with,
        abomonation_skip
    )
)]
pub fn derive(input: TokenStream) -> TokenStream {
    match derive_abomonation(input) {
        Ok(result) => result.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn derive_abomonation(input: TokenStream) -> syn::Result<TokenStream> {
    let ast: DeriveInput = syn::parse::<DeriveInput>(input)?;
    let this = Ident::new("self", Span::call_site());
    let ident = ast.ident;

    let MethodData {
        entomb,
        exhume,
        extent,
    } = match ast.data {
        syn::Data::Struct(struct_data) => derive_struct(&this, &struct_data),
        syn::Data::Enum(enum_data) => derive_enum(&this, &enum_data),
        syn::Data::Union(_) => todo!(),
    }?;

    let bound_where_clause = bounds(&ast.attrs)?;
    let generics = if omit_bounds(&ast.attrs) || bound_where_clause.is_some() {
        ast.generics
    } else {
        add_trait_bounds(ast.generics)
    };

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let where_clause = if bound_where_clause.is_some() {
        bound_where_clause.as_ref()
    } else {
        where_clause
    };

    let output = quote!(
        impl #impl_generics abomonation::Abomonation for #ident #ty_generics #where_clause {
            #[inline]
            unsafe fn entomb<W: std::io::Write>(&self, bytes: &mut W) -> std::io::Result<()> {
                #entomb
                Ok(())
            }

            #[inline]
            unsafe fn exhume<'a, 'b>(&'a mut self, mut bytes: &'b mut [u8]) -> Option<&'b mut [u8]> {
                #exhume
                Some(bytes)
            }

            #[inline]
            fn extent(&self) -> usize {
                let mut size = 0;
                #extent
                size
            }
        }
    );

    Ok(output.into())
}

fn derive_struct(this: &Ident, struct_data: &DataStruct) -> syn::Result<MethodData> {
    match &struct_data.fields {
        syn::Fields::Named(named_fields) => derive_named_fields(this, &named_fields),
        syn::Fields::Unnamed(unnamed_fields) => derive_unnamed_fields(&this, &unnamed_fields),
        syn::Fields::Unit => derive_unit_impl(),
    }
}

fn derive_enum(_this: &Ident, enum_data: &DataEnum) -> syn::Result<MethodData> {
    let mut entomb_arms = vec![];
    let mut exhume_arms = vec![];
    let mut extent_arms = vec![];
    for variant in &enum_data.variants {
        let ident = &variant.ident;
        let MethodData {
            entomb,
            exhume,
            extent,
        } = if !skip(&variant.attrs) {
            match &variant.fields {
                syn::Fields::Named(named_fields) => derive_named_fields(ident, &named_fields),
                syn::Fields::Unnamed(unnamed_fields) => {
                    derive_tuple_variant(ident, &unnamed_fields)
                }
                syn::Fields::Unit => derive_unit_impl(),
            }?
        } else {
            MethodData {
                entomb: quote!(panic!("Abomonation skipped for this variant arm")),
                exhume: quote!(panic!("Abomonation skipped for this variant arm")),
                extent: quote!(panic!("Abomonation skipped for this variant arm")),
            }
        };

        let (case, mut_case) = match get_style(variant) {
            Style::Unit => (quote!(Self::#ident), quote!(Self::#ident)),
            Style::Newtype => (
                quote!(Self::#ident(ref __field0)),
                quote!(Self::#ident(ref mut __field0)),
            ),
            Style::Tuple => {
                let field_names: Vec<_> = (0..variant.fields.len())
                    .map(|i| Ident::new(&format!("__field{}", i), Span::call_site()))
                    .collect();
                (
                    quote!(Self::#ident(#(ref #field_names),*)),
                    quote!(Self::#ident(#(ref mut #field_names),*)),
                )
            }
            Style::Struct => {
                let members: Vec<_> = variant
                    .fields
                    .iter()
                    .map(|f| f.ident.as_ref().expect("should be named field"))
                    .collect();
                (
                    quote!(Self::#ident { #(ref #members),* }),
                    quote!(Self::#ident { #(ref mut #members),* }),
                )
            }
        };

        let entomb_arm = quote!(
            #case => {
                #entomb
            },
        );
        let exhume_arm = quote!(
            #mut_case => {
                #exhume
            },
        );
        let extent_arm = quote!(
            #case => {
                #extent
            },
        );
        entomb_arms.push(entomb_arm);
        exhume_arms.push(exhume_arm);
        extent_arms.push(extent_arm);
    }

    Ok(MethodData {
        entomb: quote!(
            match *self {
                #(#entomb_arms)*
            };
        ),
        exhume: quote!(
            match *self {
                #(#exhume_arms)*
            };
        ),
        extent: quote!(
            match *self {
                #(#extent_arms)*
            };
        ),
    })
}

fn derive_named_fields(this: &Ident, named_fields: &FieldsNamed) -> syn::Result<MethodData> {
    let entomb = named_fields
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().expect("should be named field");
            derive_entomb(quote!(#this.#ident), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let exhume = named_fields
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().expect("should be named field");
            derive_exhume(quote!(#this.#ident), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let extent = named_fields
        .named
        .iter()
        .map(|field| {
            let ident = field.ident.as_ref().expect("should be named field");
            derive_extent(quote!(#this.#ident), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(MethodData {
        entomb: quote!(#(#entomb)*),
        exhume: quote!(#(#exhume)*),
        extent: quote!(#(#extent)*),
    })
}

fn derive_unnamed_fields(this: &Ident, unnamed_fields: &FieldsUnnamed) -> syn::Result<MethodData> {
    let entomb = unnamed_fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let idx = syn::Index::from(i);
            derive_entomb(quote!(#this.#idx), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let exhume = unnamed_fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let idx = syn::Index::from(i);
            derive_exhume(quote!(#this.#idx), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let extent = unnamed_fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let idx = syn::Index::from(i);
            derive_extent(quote!(#this.#idx), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(MethodData {
        entomb: quote!(#(#entomb)*),
        exhume: quote!(#(#exhume)*),
        extent: quote!(#(#extent)*),
    })
}

fn derive_tuple_variant(_this: &Ident, unnamed_fields: &FieldsUnnamed) -> syn::Result<MethodData> {
    let entomb = unnamed_fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let idx = Ident::new(&format!("__field{}", i), Span::call_site());
            derive_entomb(quote!(#idx), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let exhume = unnamed_fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let idx = Ident::new(&format!("__field{}", i), Span::call_site());
            derive_exhume(quote!(#idx), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    let extent = unnamed_fields
        .unnamed
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let idx = Ident::new(&format!("__field{}", i), Span::call_site());
            derive_extent(quote!(#idx), &field.attrs)
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(MethodData {
        entomb: quote!(#(#entomb)*),
        exhume: quote!(#(#exhume)*),
        extent: quote!(#(#extent)*),
    })
}

fn derive_unit_impl() -> syn::Result<MethodData> {
    Ok(MethodData::default())
}

fn derive_entomb(
    this: proc_macro2::TokenStream,
    attrs: &Vec<Attribute>,
) -> syn::Result<proc_macro2::TokenStream> {
    match with(attrs)? {
        Some(ty) => Ok(quote!(
            let __this = &#this as *const _ as *const #ty;
            
            (*__this).entomb(bytes)?;
        )),
        None => {
            if skip(attrs) {
                Ok(quote!())
            } else {
                Ok(quote!(#this.entomb(bytes)?;))
            }
        }
    }
}

fn derive_exhume(
    this: proc_macro2::TokenStream,
    attrs: &Vec<Attribute>,
) -> syn::Result<proc_macro2::TokenStream> {
    match with(attrs)? {
        Some(ty) => Ok(quote!(
            let __this = &mut #this as *mut _ as *mut #ty;
            bytes = (&mut *__this).exhume(bytes)?;
        )),
        None => {
            if skip(attrs) {
                Ok(quote!())
            } else {
                Ok(quote!(bytes = #this.exhume(bytes)?;))
            }
        }
    }
}

fn derive_extent(
    this: proc_macro2::TokenStream,
    attrs: &Vec<Attribute>,
) -> syn::Result<proc_macro2::TokenStream> {
    match with(attrs)? {
        Some(ty) => Ok(quote!(
            let __this = &#this as *const _ as *const #ty;
            size += unsafe { (*__this).extent() };
        )),
        None => {
            if skip(attrs) {
                Ok(quote!())
            } else {
                Ok(quote!(size += #this.extent();))
            }
        }
    }
}

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for param in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *param {
            type_param.bounds.push(parse_quote!(abomonation::Abomonation));
        }
    }
    generics
}
