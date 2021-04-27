use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::quote;
use syn::{Attribute, DataEnum, DeriveInput, Variant};

fn variant_string(variant: &Variant) -> Option<String> {
    let attribute = variant
        .attrs
        .iter()
        .find(|a| a.path.is_ident("from_string"));
    attribute.map(|a| {
        let string_literal: syn::LitStr = a
            .parse_args()
            .expect("Could not parse #[from_string(..)] attribute.");
        string_literal.value()
    })
}

fn get_macro_name(attrs: &Vec<Attribute>) -> Option<String> {
    let macro_attribute = attrs.iter().find(|a| a.path.is_ident("from_string_macro"));
    macro_attribute.map(|a| {
        let string_literal: syn::LitStr = a
            .parse_args()
            .expect("Could not parse arg of from_string_macro attribute.");
        string_literal.value()
    })
}

fn get_group_content(tokens: &TokenStream) -> TokenStream {
    tokens
        .clone()
        .into_iter()
        .map(|t| match t {
            TokenTree::Group(g) => g.stream(),
            _ => panic!("Expected a TokenTree::Group"),
        })
        .collect()
}

fn get_macro_rules(attrs: &Vec<Attribute>) -> Option<TokenStream> {
    let macro_attribute = attrs
        .iter()
        .find(|a| a.path.is_ident("from_string_macro_rules"));
    macro_attribute.map(|a| {
        // #[from_string_macro_rules( ($item:ident) => { $ident } )]
        //      We get all of this  ^~~~~~~~~~~~~~~~~~~~~~~~~~~~~~^
        //      but we only want the contents of the parenthesis.
        get_group_content(&a.tokens)
    })
}

fn map_variants<F: Fn(&Ident, &str) -> TokenStream>(
    enum_data: &DataEnum,
    map: F,
) -> Vec<TokenStream> {
    enum_data
        .variants
        .iter()
        .map(|v| {
            let variant_ident = &v.ident;
            let variant_string =
                variant_string(v).unwrap_or_else(|| v.ident.to_string().to_lowercase());
            map(variant_ident, &variant_string)
        })
        .collect()
}

fn generate_macro(input: &DeriveInput, enum_data: &DataEnum) -> Option<TokenStream> {
    let macro_name = get_macro_name(&input.attrs);
    let extra_rules = get_macro_rules(&input.attrs).unwrap_or_else(|| {
        quote! {
            ($a:ident) => { $a };
        }
    });

    macro_name.map(|name| {
        let macro_name = syn::Ident::new(&name, Span::call_site());
        let macro_rules = map_variants(enum_data, |variant_ident, variant_string| {
            quote! {
                (#variant_string) => { #macro_name!(#variant_ident) }
            }
        });

        quote! {
            #[macro_export]
            macro_rules! #macro_name {
                #extra_rules
                #(#macro_rules;)*
            }
        }
    })
}

fn generate_from_str_impl(input: &DeriveInput, enum_data: &DataEnum) -> TokenStream {
    let ident = &input.ident;
    let match_branches = map_variants(enum_data, |variant_ident, variant_string| {
        quote! {
            #variant_string => Ok(#ident::#variant_ident)
        }
    });

    quote! {
        impl std::str::FromStr for #ident {
            type Err = &'static str;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    #(#match_branches,)*
                    _ => Err("No matching enum found."),
                }
            }
        }
    }
}

pub fn enum_from_string(input: &DeriveInput, enum_data: &DataEnum) -> TokenStream {
    let from_str_impl_tokens = generate_from_str_impl(input, enum_data);
    let macro_tokens = generate_macro(input, enum_data);

    quote! {
        #macro_tokens
        #from_str_impl_tokens
    }
}
