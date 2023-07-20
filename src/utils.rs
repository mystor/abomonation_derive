use proc_macro2::Span;
use syn::{
    parse::Parse, Attribute, Error, Type,
    Variant, WhereClause,
};

#[derive(Copy, Clone)]
pub enum Style {
    /// Named fields.
    Struct,
    /// Many unnamed fields.
    Tuple,
    /// One unnamed field.
    Newtype,
    /// No fields.
    Unit,
}

pub fn get_style(variant: &Variant) -> Style {
    match &variant.fields {
        syn::Fields::Named(_) => Style::Struct,
        syn::Fields::Unnamed(fields) => {
            if fields.unnamed.len() == 1 {
                Style::Newtype
            } else {
                Style::Tuple
            }
        },
        syn::Fields::Unit => Style::Unit,
    }
}

#[inline]
pub fn with(attrs: &Vec<Attribute>) -> Result<Option<Type>, Error> {
    let fields = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("abomonate_with") {
                Some(attr.parse_args_with(Type::parse))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.len() > 1 {
        Err(Error::new(
            Span::call_site(),
            "only one with attribute is allowed",
        ))
    } else if fields.len() == 0 {
        Ok(None)
    } else {
        let field = fields[0].clone();
        Ok(Some(field))
    }
}

pub fn bounds<'a>(attrs: &Vec<Attribute>) -> Result<Option<WhereClause>, Error> {
    let fields = attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("abomonation_bounds") {
                Some(attr.parse_args_with(WhereClause::parse))
            } else {
                None
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if fields.len() > 1 {
        Err(Error::new(
            Span::call_site(),
            "only one with trait bound is allowed",
        ))
    } else if fields.len() == 0 {
        Ok(None)
    } else {
        let where_clause = fields[0].clone();
        Ok(Some(where_clause))
    }
}

pub fn omit_bounds(attrs: &Vec<Attribute>) -> bool {
    for attr in attrs.iter() {
        if attr.path().is_ident("abomonation_omit_bounds") {
            return true;
        }
    }
    false
}

pub fn skip(attrs: &Vec<Attribute>) -> bool {
    for attr in attrs.iter() {
        if attr.path().is_ident("abomonation_skip") {
            return true;
        }
    }
    false
}
