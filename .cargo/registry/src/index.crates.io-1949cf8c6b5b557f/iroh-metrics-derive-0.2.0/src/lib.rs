use heck::ToSnakeCase;
use proc_macro::TokenStream;
use quote::quote;
use syn::{
    meta::ParseNestedMeta, parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput,
    Error, Expr, ExprLit, Fields, Ident, Lit, LitStr,
};

#[proc_macro_derive(MetricsGroup, attributes(metrics))]
pub fn derive_metrics_group(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut out = proc_macro2::TokenStream::new();
    out.extend(expand_metrics(&input).unwrap_or_else(Error::into_compile_error));
    out.extend(expand_iterable(&input).unwrap_or_else(Error::into_compile_error));
    out.into()
}

#[proc_macro_derive(Iterable)]
pub fn derive_iterable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let out = expand_iterable(&input).unwrap_or_else(Error::into_compile_error);
    out.into()
}

#[proc_macro_derive(MetricsGroupSet, attributes(metrics))]
pub fn derive_metrics_group_set(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let mut out = proc_macro2::TokenStream::new();
    out.extend(expand_metrics_group_set(&input).unwrap_or_else(Error::into_compile_error));
    out.into()
}

fn expand_iterable(input: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let (name, fields) = parse_named_struct(input)?;

    let count = fields.len();

    let mut match_arms = quote! {};
    for (i, field) in fields.iter().enumerate() {
        let ident = field.ident.as_ref().unwrap();
        let ident_str = ident.to_string();
        let attr = parse_metrics_attr(&field.attrs)?;
        let help = attr
            .help
            .or_else(|| parse_doc_first_line(&field.attrs))
            .unwrap_or_else(|| ident_str.clone());
        match_arms.extend(quote! {
            #i => Some(::iroh_metrics::MetricItem::new(#ident_str, #help, &self.#ident as &dyn ::iroh_metrics::Metric)),
        });
    }

    Ok(quote! {
        impl ::iroh_metrics::iterable::Iterable for #name {
            fn field_count(&self) -> usize {
                #count
            }

            fn field_ref(&self, n: usize) -> Option<::iroh_metrics::MetricItem<'_>> {
                match n {
                    #match_arms
                    _ => None,
                }
            }
        }
    })
}

fn expand_metrics(input: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let (name, _fields) = parse_named_struct(input)?;
    let attr = parse_metrics_attr(&input.attrs)?;
    let name_str = attr
        .name
        .unwrap_or_else(|| name.to_string().to_snake_case());

    Ok(quote! {
        impl ::iroh_metrics::MetricsGroup for #name {
            fn name(&self) -> &'static str {
                #name_str
            }
        }
    })
}

fn expand_metrics_group_set(input: &DeriveInput) -> Result<proc_macro2::TokenStream, Error> {
    let (name, fields) = parse_named_struct(input)?;
    let attr = parse_metrics_attr(&input.attrs)?;
    let name_str = attr
        .name
        .unwrap_or_else(|| name.to_string().to_snake_case());

    let mut cloned = quote! {};
    let mut refs = quote! {};
    for field in fields {
        let name = field.ident.as_ref().unwrap();
        cloned.extend(quote! {
            self.#name.clone() as ::std::sync::Arc<dyn ::iroh_metrics::MetricsGroup>,
        });
        refs.extend(quote! {
            &*self.#name as &dyn ::iroh_metrics::MetricsGroup,
        });
    }

    Ok(quote! {
        impl ::iroh_metrics::MetricsGroupSet for #name {
            fn name(&self) -> &'static str {
                #name_str
            }

            fn groups_cloned(&self) -> impl ::std::iter::Iterator<Item = ::std::sync::Arc<dyn ::iroh_metrics::MetricsGroup>> {
                [#cloned].into_iter()
            }

            fn groups(&self) -> impl ::std::iter::Iterator<Item = &dyn ::iroh_metrics::MetricsGroup> {
                [#refs].into_iter()
            }
        }
    })
}

fn parse_doc_first_line(attrs: &[Attribute]) -> Option<String> {
    attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .flat_map(|attr| attr.meta.require_name_value())
        .find_map(|name_value| {
            let Expr::Lit(ExprLit { lit, .. }) = &name_value.value else {
                return None;
            };
            let Lit::Str(str) = lit else { return None };
            Some(str.value().trim().to_string())
        })
}

#[derive(Default)]
struct MetricsAttr {
    name: Option<String>,
    help: Option<String>,
}

fn parse_metrics_attr(attrs: &[Attribute]) -> Result<MetricsAttr, syn::Error> {
    let mut out = MetricsAttr::default();
    for attr in attrs.iter().filter(|attr| attr.path().is_ident("metrics")) {
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("name") {
                out.name = Some(parse_lit_str(&meta)?);
                Ok(())
            } else if meta.path.is_ident("help") {
                out.help = Some(parse_lit_str(&meta)?);
                Ok(())
            } else {
                Err(meta.error("The `metrics` attribute supports only `name` and `help` fields."))
            }
        })?;
    }
    Ok(out)
}

fn parse_lit_str(meta: &ParseNestedMeta<'_>) -> Result<String, Error> {
    let s: LitStr = meta.value()?.parse()?;
    Ok(s.value().trim().to_string())
}

fn parse_named_struct(input: &DeriveInput) -> Result<(&Ident, &Fields), Error> {
    match &input.data {
        Data::Struct(data) if matches!(data.fields, Fields::Named(_)) => {
            Ok((&input.ident, &data.fields))
        }
        _ => Err(Error::new(
            input.span(),
            "The `MetricsGroup` and `Iterable` derives support only structs.",
        )),
    }
}
