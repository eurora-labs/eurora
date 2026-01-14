// use agent_chain::{BaseMessage, HumanMessage};
// use be_remote_db::{Message, MessageType};

// pub fn convert_db_message_to_base_message(message: Message) -> BaseMessage {
//     match message.message_type {
//         MessageType::Human => BaseMessage::Human(HumanMessage {
//             id: Some(message.id.to_string()),

//         })
//         _ => todo!()
//     };
// }
