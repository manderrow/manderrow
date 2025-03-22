use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use serde_json::json;
use syn::{Data, DeriveInput, Error, Expr, Ident, Token, Type, parse::Parse, token::Eq};

struct SettingsArgs {
    sections: Vec<Ident>,
}

impl Parse for SettingsArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let span = input.span();
        let mut sections = None::<Vec<_>>;
        while !input.is_empty() {
            match input.parse::<Ident>()? {
                key if key == "sections" => {
                    input.parse::<Eq>()?;
                    let sections_buf;
                    syn::bracketed!(sections_buf in input);
                    sections = Some(
                        sections_buf
                            .parse_terminated(Ident::parse, Token![,])?
                            .into_iter()
                            .collect(),
                    );
                }
                key => return Err(Error::new(key.span(), "Unrecognized argument")),
            }
        }
        Ok(Self {
            sections: sections
                .ok_or_else(|| Error::new(span, "Missing required attribute `sections`"))?,
        })
    }
}

struct Field {
    ident: Ident,
    ty: Type,
    section: Ident,
    default: Expr,
}

#[proc_macro_attribute]
pub fn settings(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as SettingsArgs);
    let input = syn::parse_macro_input!(input as DeriveInput);
    let Data::Struct(data) = input.data else {
        panic!("wrong type of data");
    };

    let fields = data
        .fields
        .into_iter()
        .map(|field| {
            let ident = field.ident.unwrap();
            Ok(Field {
                ty: field.ty,
                section: field
                    .attrs
                    .iter()
                    .find(|attr| {
                        attr.path()
                            .get_ident()
                            .map_or(false, |ident| ident == "section")
                    })
                    .ok_or_else(|| {
                        Error::new(ident.span(), "Missing required attribute `section`")
                    })?
                    .parse_args()
                    .and_then(|section| {
                        if !args.sections.contains(&section) {
                            Err(Error::new(section.span(), "Unrecognized section. Perhaps you forgot to include it in the sections list?"))
                        } else {
                            Ok(section)
                        }
                    })?,
                default: field
                    .attrs
                    .iter()
                    .find(|attr| {
                        attr.path()
                            .get_ident()
                            .map_or(false, |ident| ident == "default")
                    })
                    .ok_or_else(|| {
                        Error::new(ident.span(), "Missing required attribute `default`")
                    })?
                    .parse_args()?,
                ident,
            })
        })
        .collect::<Result<Vec<_>, Error>>();
    let fields = match fields {
        Ok(t) => t,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let (field_ident, field_ty): (Vec<_>, Vec<_>) =
        fields.iter().map(|f| (&f.ident, &f.ty)).unzip();

    let field_accessor_ident = Ident::new("x", Span::call_site());

    let (field_accessor_bind, field_default): (Vec<_>, Vec<_>) = fields
        .iter()
        .map(|f| {
            (
                match &f.ty {
                    Type::Path(p) if p.path.get_ident().map_or(false, |ident| ident == "bool") => {
                        quote! {
                            #field_accessor_ident
                        }
                    }
                    _ => quote! {
                        ref #field_accessor_ident
                    },
                },
                &f.default,
            )
        })
        .unzip();

    let name = input.ident;

    let defaulted = format_ident!("Defaulted{name}");
    let patch = format_ident!("{name}Patch");

    let ui_ident: Ident = Ident::new("UI", Span::call_site());

    let ui = serde_json::to_string(&json!({
        "sections": args.sections.iter().map(|section| {
            json!({
                "id": cruet::to_camel_case(&section.to_string()),
                "settings": fields.iter()
                    .filter(|field| field.section == *section)
                    .map(|field| {
                        json!({
                            "key": cruet::to_camel_case(&field.ident.to_string()),
                            "input": match &field.ty {
                                Type::Path(p) if p.path.get_ident().map_or(false, |ident| ident == "bool") => json!({ "type": "Toggle" }),
                                _ => todo!(),
                            },
                        })
                    })
                    .collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>()
    })).unwrap();

    let expanded = quote! {
        #[derive(Debug, Clone, Default)]
        pub struct #name {
            #(#field_ident: Option<#field_ty>),*
        }

        #[derive(Debug, Clone, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(non_snake_case)]
        pub struct #defaulted {
            #(#field_ident: Setting<#field_ty>),*
        }

        #[derive(Debug, Clone, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(non_snake_case)]
        pub struct #patch {
            #(#[serde(default)]
            #field_ident: Option<Change<#field_ty>>),*
        }

        impl #name {
            #(pub fn #field_ident(&self) -> Setting<#field_ty> {
                match self.#field_ident {
                    Some(#field_accessor_bind) => Setting { value: #field_accessor_ident, is_default: false },
                    None => Setting { value: #field_default, is_default: true },
                }
            })*

            pub fn defaulted(&self) -> #defaulted {
                #defaulted {
                    #(#field_ident: self.#field_ident()),*
                }
            }

            pub fn update(&mut self, patch: #patch) {
                #(
                    if let Some(change) = patch.#field_ident {
                        self.#field_ident = match change {
                            Change::Default => None,
                            Change::Override(value) => Some(value),
                        };
                    }
                )*
            }
        }

        pub const #ui_ident: &str = #ui;
    };

    TokenStream::from(expanded)
}
