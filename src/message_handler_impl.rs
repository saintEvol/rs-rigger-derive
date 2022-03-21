use quote::__private::TokenStream;
use quote::quote;
use syn::{FnArg, Ident, ImplItemMethod, ItemImpl, Type};
use syn::ImplItem;

const QUESTION_ATTR_NAME: &str = "question";
const BROADCAST_ATTR_NAME: &str = "broadcast";
const TELL_ATTR_NAME: &str = "tell";

enum MessageType {
    None, // 非有效的消息处理函数类型
    Question,
    Tell,
    Broadcast
}

enum FunctionType {
    Fn,
    FnMut,
    FnOnce,
    StaticFn
}

struct MessageHandlerToken {
    pub questions: Vec<TokenStream>,
    pub tells: Vec<TokenStream>,
    pub broadcasts: Vec<TokenStream>,
}

pub(crate) fn gen_impl(item: ItemImpl) -> TokenStream{
    let self_type = &item.self_ty;
    // let all = item.items;

    let info = gen_handle_block(&item, &item.items);
    let questions = info.questions;
    let tell = info.tells;
    let broadcast = info.broadcasts;

    quote! {
        impl rs_rigger::gen_server::GenServerMessagePart for #self_type {
            fn on_question(&mut self, handler: bastion::message::MessageHandler<()>) -> bastion::message::MessageHandler<()> {
                #(#questions)*
                handler
            }

            fn on_tell(&mut self, handler: bastion::message::MessageHandler<()>) -> bastion::message::MessageHandler<()> {
                #(#tell)*
                handler
            }

            fn on_broadcast(&mut self, handler: bastion::message::MessageHandler<()>) -> bastion::message::MessageHandler<()> {
                #(#broadcast)*
                handler
            }
        }
    }
}

fn gen_handle_block(impl_item: &ItemImpl, items: &Vec<ImplItem>) -> MessageHandlerToken {
    let mut questions = vec![];
    let mut tells = vec![];
    let broadcasts = vec![];

    for it in items {
        match it {
            ImplItem::Method(method) => {
                match get_message_handler_type(method) {
                    MessageType::Question => {
                        questions.push(gen_one_question(&impl_item.self_ty, method))
                    }
                    MessageType::Tell => {
                        tells.push(gen_one_tell(&impl_item.self_ty, method))
                    }
                    MessageType::Broadcast => {

                    }
                    _ => {

                    }

                }
            },
            _ => {

            }

        }

    }

    MessageHandlerToken {
        questions,
        tells,
        broadcasts,
    }
}

fn gen_one_question(st: &Box<Type>, m: &ImplItemMethod) -> TokenStream {
    // #[doc]
    /* 消息处理函数的参数列表可能的情况：
    1. &[mut] self, MsgType, Sender
    2. &[mut], MsgType,
    3. MsgType, Sender
    4. MsgType
    暂时先只判断数量
     */
    match get_function_type(m){
        FunctionType::FnOnce => panic!("A question handler should not be a FnOnce: {}", m.sig.ident.to_string()),
        FunctionType::Fn | FunctionType::FnMut => {
            let (t, id) = parse_message_info(FunctionType::Fn, m);
            quote! {
                let handler = handler.on_question(|question: #t, sender|{
                    self.#id(question, sender)
                });
            }
        }
        FunctionType::StaticFn => {
            let (t, id) = parse_message_info(FunctionType::StaticFn, m);
            quote! {
                let handler = handler.on_question(|question: #t, sender|{
                    #st::#id(question, sender)
                });
            }
        }
    }
}

fn gen_one_tell(st: &Box<Type>, m: &ImplItemMethod) -> TokenStream {
    // #[doc]
    /* 消息处理函数的参数列表可能的情况：
    1. &[mut] self, MsgType, RefAddr
    2. &[mut], MsgType,
    3. MsgType, RefAddr
    4. MsgType
    暂时先只判断数量
     */
    match get_function_type(m){
        FunctionType::FnOnce => panic!("A tell handler should not be a FnOnce: {}", m.sig.ident.to_string()),
        FunctionType::Fn | FunctionType::FnMut => {
            let (t, id) = parse_message_info(FunctionType::Fn, m);
            quote! {
                let handler = handler.on_tell(|msg: #t, addr|{
                    self.#id(msg, addr)
                });
            }
        }
        FunctionType::StaticFn => {
            let (t, id) = parse_message_info(FunctionType::StaticFn, m);
            quote! {
                let handler = handler.on_tell(|msg: #t, addr|{
                    #st::#id(msg, addr)
                });
            }
        }
    }

}

fn parse_message_info<'a>(fun_type: FunctionType, m: &'a ImplItemMethod) ->(&'a Box<Type>, &'a Ident) {
    match fun_type {
        FunctionType ::Fn | FunctionType :: FnMut => {
            let input_len = m.sig.inputs.len();
            if (input_len != 2) && (input_len != 3) {
                panic!("A message handler shoud have only 2 or 3 arguemnts including [self]: {} => {} => {}", m.sig.ident.to_string(), input_len, ((input_len != 2) || (input_len != 3)))
            }
            // 第二个参数类型为消息的类型
            let msg_type =
                if let FnArg::Typed(msg_arg) =  &m.sig.inputs[1] {
                    &msg_arg.ty
                } else {
                    panic!("Error Msg Type: Not a typed args => {}", m.sig.ident.to_string())
                };

            let fid = &m.sig.ident;

            (msg_type, fid)
        }
        FunctionType::StaticFn => {
            let input_len = m.sig.inputs.len();
            if input_len != 1 || input_len != 2 {
                panic!("A static message handler shoud have only 1 or 2 arguemnts: {} => {}", m.sig.ident.to_string(), input_len)
            }
            // 第一个参数类型为消息的类型
            let msg_type =
                if let FnArg::Typed(msg_arg) =  &m.sig.inputs[0] {
                    &msg_arg.ty
                } else {
                    panic!("Error Msg Type: Not a typed args => {}", m.sig.ident.to_string())
                };

            let fid = &m.sig.ident;

            (msg_type, fid)

        }
        _ => panic!("Invalid Function Type for Message Handler, invalid: Fn, FnMut, StaticFn => {}", m.sig.ident.to_string())
    }

}

// 获取函数的类型
fn get_function_type(m: &ImplItemMethod) -> FunctionType {
    let inputs = &m.sig.inputs;
    for f in inputs {
        match f {
            FnArg::Receiver(r) =>{
                if let Some(_) = r.reference {
                    if let Some(_) = r.mutability {
                        return FunctionType::FnMut
                    } else {
                        return FunctionType::Fn
                    }
                } else {
                    return FunctionType::FnOnce
                }
            },
            _ => continue
        }
    }

    FunctionType::StaticFn
}

fn get_message_handler_type(m: &ImplItemMethod) -> MessageType {
    let len = m.attrs.len();
    if len <= 0 {
        return MessageType::None;
    }

    for a in &m.attrs {
        let seg_len = a.path.segments.len();
        if seg_len <= 0 {
            return MessageType::None;
        }

        let last_part = a.path.segments[seg_len - 1].ident.to_string();
        if last_part.eq(QUESTION_ATTR_NAME) {
            return MessageType::Question;
        } else if last_part.eq(TELL_ATTR_NAME) {
            return MessageType::Tell;

        } else if last_part.eq(BROADCAST_ATTR_NAME) {
            return MessageType::Broadcast;
        }
    }

    MessageType::None
}