package yn_server

import (
	"errors"
	"fmt"

	"github.com/google/uuid"
	"wilkuu.xyz/yapnet_v1/protocol"
)

type ChatID string 
type GroupID string 

const (
	GIDAny = "any" 
) 


type GroupOrPlayer struct { 
	Player_token uuid.UUID 
	Group_name   GroupID 
} 

// 
func (g GroupOrPlayer) IsPlayer() bool {
	return g.Player_token != uuid.Nil  
} 

type GameState struct {
	Players map[uuid.UUID]PlayerState
  Chats map[ChatID]Chat
} 


type PlayerState struct {
	Online bool
	Username string
	Groups []GroupID  
}


func (st *GameState) addPlayer(u uuid.UUID, username string) error {

	for uuid,playerst := range st.Players {
		if uuid == u || playerst.Username == username {
			return errors.New(fmt.Sprintf("Cannot add user, username '%v' or uuid is not unique!", username))
		} 
	}  

	st.Players[u] = PlayerState {
		Online: true, 
		Username: username,
		Groups: []GroupID{GroupID(GIDAny)}, // TODO: assign roles?
	} 
	return nil 
}


func newState() GameState {
	return GameState{
		Players: make(map[uuid.UUID]PlayerState),
			Chats: map[ChatID]Chat{
				ChatID("general"): {
					ChatMessages: make([]protocol.Message,0),
					ControlMessages: make([]protocol.Message,0),
					CurrentAccess: []GroupOrPlayer{
					{ Group_name: GIDAny},
				}, 
			},
		},
	}	
} 
