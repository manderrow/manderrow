use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Ident, Path, Result, Token, Type,
    parse::Parse,
    spanned::Spanned,
    token::{Comma, Eq},
};

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
    input: Ident,
    /// The values that a `select` input offers
    options: Option<Vec<Ident>>,
    ref_by_ty: Type,
    ref_by_fn: Path,
}

fn try_parse_attribute<T: Parse>(current: Option<(Span, T)>, attr: Attribute) -> Result<(Span, T)> {
    if let Some((span, _)) = current {
        let mut e = Error::new(attr.path().span(), "Duplicate attribute");
        e.combine(Error::new(span, "The first attribute is here"));
        return Err(e);
    }

    Ok((attr.span(), attr.parse_args()?))
}

fn expect_attribute<T>(ident: &Ident, name: &str, attribute: Option<(Span, T)>) -> Result<T> {
    match attribute {
        Some((_, t)) => Ok(t),
        None => Err(Error::new(
            ident.span(),
            format!("Missing required attribute `{name}`"),
        )),
    }
}

struct OptionsAttrArgs(Vec<Ident>);

impl Parse for OptionsAttrArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self(
            input
                .parse_terminated(Ident::parse, Token![,])?
                .into_iter()
                .collect(),
        ))
    }
}

struct RefByAttrArgs(Type, Path);

impl Parse for RefByAttrArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let ty = input.parse()?;
        input.parse::<Comma>()?;
        let func = input.parse()?;
        Ok(Self(ty, func))
    }
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

            let mut section = None;
            let mut default = None;
            let mut input = None;
            let mut options = None;
            let mut ref_by = None;

            for attr in field.attrs {
                match attr.path().get_ident() {
                    Some(ident) if ident == "section" => {
                        let (span, ident) = try_parse_attribute(section, attr)?;
                        if !args.sections.contains(&ident) {
                            return Err(Error::new(
                                ident.span(),
                                "Unrecognized section. Perhaps you forgot to include it in the sections list?",
                            ));
                        }
                        section = Some((span, ident));
                    }
                    Some(ident) if ident == "default" => {
                        default = Some(try_parse_attribute(default, attr)?);
                    }
                    Some(ident) if ident == "input" => {
                        input = Some(try_parse_attribute(input, attr)?);
                    }
                    Some(ident) if ident == "options" => {
                        options = Some(try_parse_attribute(options, attr)?);
                    }
                    Some(ident) if ident == "ref_by" => {
                        ref_by = Some(try_parse_attribute(ref_by, attr)?);
                    }
                    // documentation for the field
                    Some(ident) if ident == "doc" => {}
                    _ => return Err(Error::new(attr.path().span(), "Unrecognized attribute")),
                }
            }

            let RefByAttrArgs(ref_by_ty, ref_by_fn) = expect_attribute(&ident, "ref_by", ref_by)?;
            let input: Ident = expect_attribute(&ident, "input", input)?;
            let options = options.map(|(_, OptionsAttrArgs(options))| options);
            if (input == "select") != options.is_some() {
                return Err(Error::new(
                    input.span(),
                    "The `options` attribute is required by `select` inputs, and only by them",
                ));
            }

            Ok(Field {
                ty: field.ty,
                section: expect_attribute(&ident, "section", section)?,
                default: expect_attribute(&ident, "default", default)?,
                input,
                options,
                ref_by_ty,
                ref_by_fn,
                ident,
            })
        })
        .collect::<Result<Vec<_>>>();
    let fields = match fields {
        Ok(t) => t,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let (field_ident, field_ty): (Vec<_>, Vec<_>) =
        fields.iter().map(|f| (&f.ident, &f.ty)).unzip();

    let field_accessor_ident = Ident::new("x", Span::call_site());

    let (field_accessor_by_ref, field_by_ref_ty): (Vec<_>, Vec<_>) = fields
        .iter()
        .map(|f| {
            let ref_by_fn = &f.ref_by_fn;
            (
                quote! {
                    #ref_by_fn(#field_accessor_ident)
                },
                &f.ref_by_ty,
            )
        })
        .unzip();

    let field_default: Vec<_> = fields.iter().map(|f| &f.default).collect();
    let field_key: Vec<_> = fields
        .iter()
        .map(|f| cruet::to_camel_case(&f.ident.to_string()))
        .collect();

    let name = input.ident;

    let patch = format_ident!("{name}Patch");

    let sections = args.sections.iter().map(|section| {
        let id = cruet::to_camel_case(&section.to_string());
        let settings = fields
            .iter()
            .filter(|field| field.section == *section)
            .map(|field| {
                let key = cruet::to_camel_case(&field.ident.to_string());
                let input = format_ident!("{}", cruet::to_class_case(&field.input.to_string()));
                let options = field
                    .options
                    .iter()
                    .flatten()
                    .map(|option| option.to_string());
                quote! {
                    SettingInfo { key: #key, input: Input::#input, options: &[#(#options),*] }
                }
            });
        quote! {
            SectionInfo { id: #id, settings: &[#(#settings),*] }
        }
    });

    let expanded = quote! {
        #[derive(Debug, Clone, Default)]
        pub struct #name {
            #(#field_ident: Option<#field_ty>),*
        }

        #[derive(Debug, Clone, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        #[allow(non_snake_case)]
        pub struct #patch {
            #(#[serde(default)]
            #field_ident: Option<Change<#field_ty>>),*
        }

        impl #name {
            #(pub fn #field_ident<'a>(&'a self) -> Setting<#field_by_ref_ty> {
                match self.#field_ident {
                    Some(ref #field_accessor_ident) => Setting { value: #field_accessor_by_ref, is_default: false },
                    None => Setting { value: #field_default, is_default: true },
                }
            })*

            /// A setting's value as JSON, by its key, and whether it's the default.
            pub fn json(&self, key: &str) -> Option<(serde_json::Value, bool)> {
                match key {
                    #(#field_key => {
                        let setting = self.#field_ident();
                        Some((serde_json::to_value(setting.value).ok()?, setting.is_default))
                    })*
                    _ => None,
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

        /// The sections of the settings, with what each setting is set with.
        pub const SECTIONS: &[SectionInfo] = &[#(#sections),*];
    };

    TokenStream::from(expanded)
}
