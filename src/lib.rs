use quote::quote;
use syn::{ImplItem, ItemImpl, parse_macro_input, ReturnType, Type};
use syn::FnArg::Typed;
use proc_macro::{TokenStream};
use crate::message_handler_impl::gen_impl;

pub(crate) mod message_handler_impl;
// pub mod traits;

#[proc_macro_attribute]
pub fn message_block(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut item = parse_macro_input!(item as ItemImpl);
    let t = item.self_ty.clone();
    let all = item.items.clone();

    let mut print_token = vec![];
    let mut question_token = vec![];
    for it in &all {
        match it {
            ImplItem::Method(m) => {
                let attr_len = m.attrs.len();
                let (seg_len, attr_name) =
                    if attr_len > 0 {
                        let attr = &m.attrs[0];
                        (attr.path.segments.len(), attr.path.segments[attr.path.segments.len() - 1].ident.to_string())
                    } else {
                        (0, "".to_string())
                    };
                // 生成消息处理函数
                if m.sig.inputs.len() == 3 {
                    match  m.sig.inputs[1].clone() {
                        Typed(p) => {
                            let t = p.ty;
                            let id = m.sig.ident.clone();
                            question_token.push(
                                quote! {
                                    let handler = handler.on_question(|question: #t, sender|{
                                        self.#id(question, sender);
                                    });
                                }
                            );
                        }
                        _ => ()
                    };
                }

                let mn_str = m.sig.ident.to_string();

                let output_str = match &m.sig.output {
                    ReturnType::Default => {
                        "default".to_string()
                    }
                    ReturnType::Type(_, t) => {
                        match &**t {
                            Type::Verbatim(ref token) => token.to_string(),
                            _ =>  "unknown".to_string()
                        }
                    }

                };
                print_token.push(quote! {
                    println!("method: name => {}, attr len => {}, attr => {}, seg_path_len => {}, return => {}", #mn_str, #attr_len, #attr_name, #seg_len, #output_str)

                })
            }
            _ => {
                print_token.push(quote! {
                    println!("unknown impl item")
                })
            }
        }
    }

    let message_handler_impl = gen_impl(item.clone());
    let final_token = quote!{
        #item

        impl #t {
            pub fn print_all() {
                println!("test +++");
                #(
                    #print_token;
                )*
                // for ref it in #all {
                //
                // }
            }
        }

        #message_handler_impl
    };

    proc_macro::TokenStream::from(final_token)

}

#[proc_macro_attribute]
pub fn question(_args: TokenStream, input: TokenStream) -> TokenStream {

    proc_macro::TokenStream::from(input)
}

#[proc_macro_attribute]
pub fn tell(args: TokenStream, input: TokenStream) -> TokenStream {

    proc_macro::TokenStream::from(input)
}

#[proc_macro_attribute]
pub fn broadcast(args: TokenStream, input: TokenStream) -> TokenStream {

    proc_macro::TokenStream::from(input)
}
