package yn_server

import (
	"errors"
	"fmt"

	"github.com/google/uuid"
	"wilkuu.xyz/yapnet/protocol"
)

type ChatID string // Identifier for a chat
type GroupID string // Identifier for a group

const (
	// Specified group id for anyone
	GIDAny = "any"
)

type GroupOrPlayer struct {
	Player_token uuid.UUID
	Group_name   GroupID
}

func (g GroupOrPlayer) IsPlayer() bool {
	return g.Player_token != uuid.Nil
}

type GameState struct {
	Players map[uuid.UUID]*PlayerState
	Chats   map[ChatID]Chat
	ServerMessages []protocol.Message 
}

type PlayerState struct {
	Online   bool
	Username string
	Groups   []GroupID
}

func (st *GameState) addPlayer(u uuid.UUID, username string) error {

	for uuid, playerst := range st.Players {
		if uuid == u || playerst.Username == username {
			return errors.New(fmt.Sprintf("Cannot add user, username '%v' or uuid is not unique!", username))
		}
	}

	st.Players[u] = &PlayerState{
		Online:   true,
		Username: username,
		Groups:   []GroupID{GroupID(GIDAny)}, // TODO: assign roles?
	}
	return nil
}

func newState() GameState {
	return GameState{
		Players: make(map[uuid.UUID]*PlayerState),
		Chats: map[ChatID]Chat{
			ChatID("general"): {
				ChatMessages:    make([]protocol.Message, 0),
				ControlMessages: make([]protocol.Message, 0),
				CurrentAccess: []GroupOrPlayer{
					{Group_name: GIDAny},
				},
			},
		},
		ServerMessages: make([]protocol.Message, 0),
	}
}

func (s * GameState) AddServerMessage(m protocol.Message) {

	// NOTE: Idk why i need to assign it to a variable
	// but if i inline this, the slice only contains the last message
	msgs := append(s.ServerMessages, m)
	s.ServerMessages = msgs
}
