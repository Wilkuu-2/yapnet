package protocol

import "github.com/google/uuid"

const (
	RECAP_CHUNK_SIZE int64 = 64
)

// An introduction packet that marks the begin of the exchange.
// Sent by the client.
type HelloMessage struct {
	Name     string   `json:"name"`
	Versions []string `json:"versions"`
}

func (HelloMessage) MsgType() MsgType { return MsgTypeHello }

// An reintroduction packet that reauthenticates the user using the token. 
// Sent by the client. 
type BackMessage struct {
	Token    uuid.UUID `json:"token"`
	Versions []string  `json:"versions"`

}
func (BackMessage) MsgType() MsgType { return MsgTypeBack }


// A answer to a Hello or a Back packet. It fills the user in on all the authentication information.  
// Sent by the server. 
type WelcomeMessage struct {
	Name    string    `json:"name"`
	Token   uuid.UUID `json:"token"`
	Version string    `json:"version" `
}

func (WelcomeMessage) MsgType() MsgType { return MsgTypeWelcome }


// Starts the recap procedure by defining how many messages there will be and how big the chunks are. 
type RecapStart struct{
	MessagesCount int32
	ChunkSize int32 
}
func (RecapStart) MsgType() MsgType { return MsgTypeRecapHead }

type RecapTail struct {
	start int64 
	msgs []Message
}
func (RecapTail) MsgType() MsgType { return MsgTypeRecapTail }


