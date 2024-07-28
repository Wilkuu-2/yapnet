package protocol

import (
	"encoding/json"
)

// Sequence number of a message, establishes order of messages. 
type SeqType uint64

// An inner data of a message
type MessageData interface {
	MsgType() MsgType
}

// A raw message parsed enough to know which kind of message it is. 
// Recieved from the client. 
type RawMessage struct {
	Msg_type MsgType          `json:"msg_type"`
	Seq      SeqType          `json:"seq"`
	Data     *json.RawMessage `json:"data"`
}

// A parsed message with the data filled by the appropriate struct.
// Often used to sent a message to the client. 
type Message struct {
	Msg_type MsgType      `json:"msg_type"`
	Seq      SeqType      `json:"seq"`
	Data     *MessageData `json:"data"`
}


// Wrap a MessageData struct in a Message for sending to the client. 
func Msg(d MessageData) *Message {
	return &Message{
 		Msg_type: d.MsgType(),
 		Data:     &d,
 	}
 }

