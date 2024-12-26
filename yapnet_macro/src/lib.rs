use darling::{FromDeriveInput, FromMeta, FromMetaItem};
use proc_macro::{self, TokenStream};
use proc_macro2::Span;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{End, Parse, ParseStream}, parse_macro_input, spanned::Spanned, token::Token, DeriveInput, Expr, GenericParam, Ident, Item, LitStr, Meta, MetaNameValue, Path, Token, Type
};

#[derive(FromDeriveInput, Default)]
#[darling(default, attributes(msg_data), forward_attrs(allow, doc, cfg))]
struct Opts {
    msg_type: String,
    global: bool,
}

fn get_lit_string(e: Expr) -> syn::Result<String> {
    match e {
        Expr::Lit(l) => match l.lit {
          syn::Lit::Str(s) => Ok(s.value()), 
          _ => { Err(syn::Error::new_spanned(l, "This should be a string literal"))},
        },
        _ => Err(syn::Error::new_spanned(e, "This should be a string literal"))
    } 

} 
fn get_lit_bool(e: Expr) -> syn::Result<bool> {
    match e {
        Expr::Lit(l) => match l.lit {
          syn::Lit::Bool(s) => Ok(s.value), 
          _ => { Err(syn::Error::new_spanned(l, "This should be a boolean literal"))},
        },
        _ => Err(syn::Error::new_spanned(e, "This should be a boolean literal"))
    } 
} 


fn handle_missing_arg<T>(opt: Option<T>, span: Span, name: &str)-> syn::Result<T> {
    match opt {
        Some(x) => Ok(x),
        None => Err(syn::Error::new(span, format!("Missing value: {}", name)))
    }
}

impl Parse for Opts {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut msg_type = None; 
        let mut global = None; 

        loop {
            if input.peek(End) {
                break;
            }
            
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
                continue;
            }

            if input.peek(syn::Ident) {
                let mv: MetaNameValue = input.parse()?; 

                match mv.path.segments.last().unwrap().ident.to_string().as_str() {
                    "msg_type" => msg_type = Some(get_lit_string(mv.value)?),
                    "global" => global = Some(get_lit_bool(mv.value)?),
                    _ => return Err(syn::Error::new_spanned(mv, "Foreign name_value")),
                }
            }

        }
        Ok(Self{
            msg_type: handle_missing_arg(msg_type, input.span(), stringify!(msg_type))?, 
            global: handle_missing_arg(global, input.span(), stringify!(global))?, 
        })
    }

}


#[proc_macro_derive(MessageDataV2, attributes(msg_data))]
pub fn derive_message_data(_item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(_item);
    //  TODO: Better diagnostics
    let opts = Opts::from_derive_input(&input).unwrap();
    let DeriveInput { ident, data: _, .. } = input;
    println!("Deriving MessageDataV2 for {}", ident.to_string()); 

    let msg_type = opts.msg_type;
    let global = opts.global;


    let output = quote! {
        impl crate::protocol::MessageDataV2 for #ident {
            fn msg_type(&self) -> &'static str { #msg_type }
            fn is_global(&self) -> bool { #global }

            fn subject(&self) -> Option<crate::protocol:: UserId> { None }
            fn object(&self)  -> Option<crate::protocol::UserId> { None }
            fn chat(&self)    -> Option<crate::protocol::ChatId> { None }
        }
    };
    output.into()
}

struct ProtocolItem(Ident, Type, String);

impl ToTokens for ProtocolItem {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        let mut ide = self.0.clone();
        let typ = self.1.clone();
        let msg_type = self.2.clone();
        
        ide = format_ident!("Body{}", ide);
        let out = quote! { 
            #[serde(rename = #msg_type)]
            #ide(#typ)
        };
        tokens.extend(out);
    }
}

struct ProtocolBody {
    items: Vec<ProtocolItem>,
}

impl ProtocolBody {
    fn get_idents(&self) -> Vec<Ident> {
        self.items
            .iter()
            .map(|ProtocolItem(i, _, _)| i.clone())
            .collect()
    }
    fn get_types(&self) -> Vec<Type> {
        self.items
            .iter()
            .map(|ProtocolItem(_, t, _)| t.clone())
            .collect()
    }
    fn get_enum_idents(&self) -> Vec<Ident> {
        self.items
            .iter()
            .map(|ProtocolItem(i, _, _)| format_ident!("Body{}", i.clone()))
            .collect()
    }
}

impl Parse for ProtocolBody {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut items = vec![];

        loop {
            if input.peek(End) {
                break;
            }
            let item: Item = input.parse()?;
            match &item {
                Item::Struct(st) => {
                    let ident = st.ident.clone();
                    println!("Parsing struct {}", ident.to_string()); 

                    let mut msg_type = None; 
                    for attr in st.attrs.iter() {
                        if attr.meta.path() == &Path::from_string("msg_data")?
                        { 
                            let msg_data: Opts = attr.parse_args()?;
                            msg_type = Some(msg_data.msg_type); 
                            break;
                        }
                    }
                    let mt = handle_missing_arg(msg_type, st.span(), "msg_type attribute")?; 
                    

                    let generics: Vec<GenericParam> =
                        st.generics.params.iter().map(|p| p.clone()).collect();
                    let typ;
                    if generics.len() > 0 {
                        typ = Type::Verbatim(quote! {#ident<#(#generics),*>})
                    } else {
                        typ = Type::Verbatim(quote! {#ident})
                    }
                    items.push(ProtocolItem(ident, typ, mt));
                }
                _ => {}
            }
        }

        return Ok(Self { items });
    }
}

#[proc_macro]
pub fn protocol_body(module: TokenStream) -> TokenStream {
    let m0 = module.clone();
    let input: ProtocolBody = parse_macro_input!(m0);
    let m1 = proc_macro2::TokenStream::from(module);
    let items = input.items.iter();
    let _idents = input.get_idents();
    let variants = input.get_enum_idents();
    let typs = input.get_types();

    quote! {
        #[derive(Debug,Clone,serde::Serialize, serde::Deserialize)]
        #[serde(tag= "msg_type", content="data")]
        pub enum MessageV2Enum{
            #(#items),*
        }

        impl MessageV2Enum {
           pub fn to_inner(self) -> Box<dyn crate::protocol::MessageDataV2> {
                match self {
                    #(Self::#variants(x) => Box::new(x)),*
                }
            }
           pub fn to_inner_ref<'a>(&'a self) -> &'a dyn crate::protocol::MessageDataV2 {
                match self {
                    #(Self::#variants(x) => x),*
                }
            }
        }

        #(impl From<#typs> for MessageV2Enum {
             fn from(value: #typs) -> Self {
                 Self::#variants(value)
             }
        } )*

        #(impl TryFrom<MessageV2Enum> for #typs {
            type Error = ();
            fn try_from(value: MessageV2Enum) -> Result<Self, Self::Error>{
                match value {
                    MessageV2Enum::#variants(x) => Ok(x),
                    _ => Err(())
                }
            }
        })*

        #m1

    }
    .into()
}
