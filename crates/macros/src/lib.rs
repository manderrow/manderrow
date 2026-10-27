use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Ident, Result, Token, Type, parse::Parse,
    spanned::Spanned, token::Eq,
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
    docs: Vec<Attribute>,
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

fn parse_field(field: syn::Field, sections: &[Ident]) -> Result<Field> {
    let ident = field.ident.unwrap();

    let mut section = None;
    let mut default = None;
    let mut docs = Vec::new();

    for attr in field.attrs {
        match attr.path().get_ident() {
            Some(name) if name == "section" => {
                let (span, ident) = try_parse_attribute::<Ident>(section, attr)?;
                if !sections.contains(&ident) {
                    return Err(Error::new(
                        ident.span(),
                        "Unrecognized section. Perhaps you forgot to include it in the sections list?",
                    ));
                }
                section = Some((span, ident));
            }
            Some(name) if name == "default" => {
                default = Some(try_parse_attribute(default, attr)?);
            }
            Some(name) if name == "doc" => docs.push(attr),
            _ => return Err(Error::new(attr.path().span(), "Unrecognized attribute")),
        }
    }

    Ok(Field {
        section: expect_attribute(&ident, "section", section)?,
        default: expect_attribute(&ident, "default", default)?,
        ty: field.ty,
        docs,
        ident,
    })
}

/// Makes the app's settings of a struct of them, whose fields are each a setting, of a type that
/// implements `SettingType`, which decides what the settings page sets it with:
///
/// ```ignore
/// #[manderrow_macros::settings(sections = [general])]
/// struct Settings {
///     /// Docs, which go to its getter.
///     #[section(general)]
///     #[default(true)]
///     some_toggle: bool,
/// }
/// ```
///
/// Each setting has a getter of its `Setting`, a `Key` that names it, and a variant of
/// `SettingChange`, which `Settings::apply` makes. The settings are listed by section in
/// `SECTIONS`, for the settings page. A default must be a constant.
///
/// What it makes refers to the `Setting`, `SettingType`, `Change`, `Value`, `Input`, and `Section`
/// that are in scope where it's used.
#[proc_macro_attribute]
pub fn settings(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = syn::parse_macro_input!(args as SettingsArgs);
    let input = syn::parse_macro_input!(input as DeriveInput);
    let Data::Struct(data) = input.data else {
        return Error::new(input.ident.span(), "Settings must be a struct")
            .to_compile_error()
            .into();
    };

    let fields = data
        .fields
        .into_iter()
        .map(|field| parse_field(field, &args.sections))
        .collect::<Result<Vec<_>>>();
    let fields = match fields {
        Ok(t) => t,
        Err(e) => return TokenStream::from(e.to_compile_error()),
    };

    let field_ident: Vec<_> = fields.iter().map(|f| &f.ident).collect();
    let field_ty: Vec<_> = fields.iter().map(|f| &f.ty).collect();
    let field_default: Vec<_> = fields.iter().map(|f| &f.default).collect();
    let field_docs: Vec<_> = fields.iter().map(|f| &f.docs).collect();
    let field_variant: Vec<_> = fields
        .iter()
        .map(|f| format_ident!("{}", cruet::to_class_case(&f.ident.to_string())))
        .collect();
    let field_name: Vec<_> = fields
        .iter()
        .map(|f| cruet::to_camel_case(&f.ident.to_string()))
        .collect();

    let name = input.ident;

    let sections = args.sections.iter().map(|section| {
        let id = cruet::to_camel_case(&section.to_string());
        let keys = fields
            .iter()
            .zip(&field_variant)
            .filter(|(field, _)| field.section == *section)
            .map(|(_, variant)| variant);
        quote! {
            Section { id: #id, keys: &[#(Key::#keys),*] }
        }
    });

    let expanded = quote! {
        /// The settings, which are each their default unless they've been changed.
        #[derive(Debug, Clone, Default)]
        pub struct #name {
            #(#field_ident: Option<#field_ty>),*
        }

        /// One of the settings, as the settings page lists them.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Key {
            #(#(#field_docs)* #field_variant),*
        }

        impl Key {
            /// Its name, in camelCase, which is that of its translation.
            pub const fn name(self) -> &'static str {
                match self {
                    #(Self::#field_variant => #field_name),*
                }
            }

            /// What the settings page sets it with.
            pub const fn input(self) -> Input {
                match self {
                    #(Self::#field_variant => <#field_ty as SettingType>::INPUT),*
                }
            }
        }

        /// A change to one of the settings.
        #[derive(Debug, Clone)]
        pub enum SettingChange {
            #(#field_variant(Change<#field_ty>)),*
        }

        impl SettingChange {
            /// A change to a setting, as the settings page makes it, or `None` if its value isn't
            /// of the kind the setting's input sets.
            pub fn new(key: Key, change: Change<Value>) -> Option<Self> {
                Some(match key {
                    #(Key::#field_variant => Self::#field_variant(
                        change.try_map(<#field_ty as SettingType>::from_value)?
                    )),*
                })
            }

            /// Changes a setting back to its default.
            pub fn reset(key: Key) -> Self {
                match key {
                    #(Key::#field_variant => Self::#field_variant(Change::Reset)),*
                }
            }
        }

        impl #name {
            #(
                #(#field_docs)*
                pub fn #field_ident(&self) -> Setting<&#field_ty> {
                    static DEFAULT: #field_ty = #field_default;
                    match &self.#field_ident {
                        Some(value) => Setting { value, is_default: false },
                        None => Setting { value: &DEFAULT, is_default: true },
                    }
                }
            )*

            /// A setting, as the settings page shows it.
            pub fn get(&self, key: Key) -> Setting<Value> {
                match key {
                    #(Key::#field_variant => self.#field_ident().map(<#field_ty as SettingType>::to_value)),*
                }
            }

            pub fn apply(&mut self, change: SettingChange) {
                match change {
                    #(SettingChange::#field_variant(change) => {
                        self.#field_ident = change.into_override();
                    })*
                }
            }
        }

        /// The sections of the settings, which the settings page has a tab for each of.
        pub const SECTIONS: &[Section] = &[#(#sections),*];
    };

    TokenStream::from(expanded)
}
