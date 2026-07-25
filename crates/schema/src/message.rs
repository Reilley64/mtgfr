use crate::dto::{MessageParam, MessageRef};

pub(crate) fn to_wire_message(message: engine::MessageRef) -> MessageRef {
    message.into()
}

pub(crate) fn message(key: impl Into<String>) -> MessageRef {
    MessageRef::key(key)
}

pub(crate) fn named_message(key: impl Into<String>, name: impl Into<String>) -> MessageRef {
    MessageRef::key(key).with_params(vec![MessageParam::string("name", name)])
}

pub(crate) fn child_message(key: impl Into<String>, child: MessageRef) -> MessageRef {
    MessageRef::key(key).with_children(vec![child])
}

impl From<engine::MessageRef> for MessageRef {
    fn from(value: engine::MessageRef) -> Self {
        MessageRef {
            key: value.key.as_str().to_string(),
            params: value.params.into_iter().map(MessageParam::from).collect(),
            children: value.children.into_iter().map(MessageRef::from).collect(),
        }
    }
}

impl From<engine::MessageParam> for MessageParam {
    fn from(value: engine::MessageParam) -> Self {
        let name = value.name.to_string();
        match value.value {
            engine::MessageParamValue::Str(value) => MessageParam::string(name, value),
            engine::MessageParamValue::OwnedStr(value) => MessageParam::string(name, value),
            engine::MessageParamValue::Int(value) => MessageParam::int(name, value),
            engine::MessageParamValue::Bool(value) => MessageParam::bool(name, value),
            engine::MessageParamValue::AmountToken(value) => {
                MessageParam::amount_token(name, value)
            }
        }
    }
}
