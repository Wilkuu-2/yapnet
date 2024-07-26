package protocol

// A message recieved from the client that wants to send a chat message. 
type ChatSendMessage struct {
	Chat   string `json:"chat_content"`
	Target string `json:"chat_target"`
}

func (ChatSendMessage) MsgType() MsgType { return MsgTypeChatSend }

// A message sent by the server to notify all other clients that a chat message is sent. 
type ChatSentMessage struct {
	Sender string `json:"sender"`
	Chat   string `json:"chat_content"`
	Origin string `json:"chat_target"`
}

func (ChatSentMessage) MsgType() MsgType { return MsgTypeChatSent }

// An response from the server for the client going off-protocol 
type ErrorMessage struct {
	Kind    string                 `json:"kind"`
	Info    string                 `json:"info"`
	Details map[string]interface{} `json:"details"`
}

func (ErrorMessage) MsgType() MsgType { return MsgTypeError }

// A simple echo, sent by the client and resent by the server 
type EchoMessage map[string]interface{}

func (EchoMessage) MsgType() MsgType { return MsgTypeEcho }

// A message that is not valid. Recieved from a client. 
type InvalidMessage map[string]interface{}

func (InvalidMessage) MsgType() MsgType { return MsgTypeInvalid }
