package protocol

import (
	"encoding/json"
	"errors"

	"github.com/google/uuid"
)


type SeqType uint64
type MessageData interface {
	MsgType() MsgType
}

type EchoMessage map[string]interface{}

func (EchoMessage) MsgType() MsgType { return MsgTypeEcho }

type InvalidMessage map[string]interface{}

func (InvalidMessage) MsgType() MsgType { return MsgTypeInvalid }

type Message struct {
	Msg_type MsgType     `json:"msg_type"`
	Seq      SeqType     `json:"seq"`
	Data     MessageData `json:"data"`
}

type ChatSendMessage struct {
	Chat   string `json:"chat_content"`
	Target string `json:"chat_target"`
}

func (ChatSendMessage) MsgType() MsgType { return MsgTypeChatSend }

type ErrorMessage struct {
	Kind    string                 `json:"kind"`
	Info    string                 `json:"info"`
	Details map[string]interface{} `json:"details"`
}

func (ErrorMessage) MsgType() MsgType { return MsgTypeError }

type HelloMessage struct {
	Name     string   `json:"name"`
	Versions []string `json:"versions"`
}

func (HelloMessage) MsgType() MsgType { return MsgTypeHello }

type BackMessage struct {
	Token    uuid.UUID `json:"token"`
	Versions []string  `json:"versions"`
}

func (BackMessage) MsgType() MsgType { return MsgTypeBack }

type WelcomeMessage struct {
	Name    string    `json:"name"`
	Token   uuid.UUID `json:"token"`
	Version string    `json:"version" `
}

func (WelcomeMessage) MsgType() MsgType { return MsgTypeWelcome }

type ChatSentMessage struct {
	Sender string `json:"sender"`
	Chat   string `json:"chat_content"`
	Origin string `json:"chat_target"`
}

func (ChatSentMessage) MsgType() MsgType { return MsgTypeChatSent }

type ChatRecapMessage struct{}

func (ChatRecapMessage) MsgType() MsgType { return MsgTypeChatRecap }

func Msg(d MessageData) * Message {
	return &Message{
		Msg_type: d.MsgType(),
		Data:     d,
	}
}

func logBytes(data []byte) {
	str := string(data[:])
	println(str)
}

func (m * Message) UnmarshalJSON(data []byte) error {
	var objMap map[string]json.RawMessage
	err := json.Unmarshal(data, &objMap)
	if err != nil {
		return err
	}
	msg_type, ok := objMap["msg_type"]
	if !ok {
		return errors.New("Unable to find 'msg_type' field in message")
	}

	err = json.Unmarshal(msg_type, &m.Msg_type)
	if err != nil {
		return err
	}

	msg_data, ok := objMap["data"]

	if !ok {
		return errors.New("Unable to find 'data' field in message")
	}
	switch m.Msg_type {
	case MsgTypeHello:
		var d HelloMessage
		if err := json.Unmarshal(msg_data, &d); err != nil {
			return err
		}
		m.Data = d
	case MsgTypeChatSend:
		var d ChatSendMessage
		if err := json.Unmarshal(msg_data, &d); err != nil {
			return err
		}
		m.Data = d
	case MsgTypeEcho:
		var d EchoMessage
		if err := json.Unmarshal(msg_data, &d); err != nil {
			return err
		}
		m.Data = d
	case MsgTypeBack:
		var d BackMessage
		if err := json.Unmarshal(msg_data, &d); err != nil {
			return err
		}
		m.Data = d
	default:
		var d InvalidMessage
		if err := json.Unmarshal(msg_data, &d); err != nil {
			return err
		}
		m.Data = d
	}

	return nil
}
