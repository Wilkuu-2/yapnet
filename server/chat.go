package yn_server

import (
	"encoding/json"
	"errors"

	"github.com/google/uuid"
	"wilkuu.xyz/yapnet/protocol"
)

func (s Server) handleChat(m *RawClientMessage) {
	var chat_msg protocol.ChatSendMessage
	err := json.Unmarshal([]byte(*m.msg.Data), &chat_msg)
	if err != nil {
		s.Log.Logf("Malformed message: %v", err)
	}

	// chat_msg := m.msg.Data.(protocol.ChatSendMessage)
	s.Log.Logf("Handling chat message from %v!", m.client.conn.RemoteAddr())
	s.Log.Logf("%+v!", chat_msg)

	player_id, ok := s.clients[m.client]
	if !ok || player_id == uuid.Nil {
		m.client.send <- protocol.Msg(protocol.ErrorMessage{
			Kind:    "NotLoggedIn",
			Info:    "You are not logged in, so you cannot chat.\nYour client has not sent a Hello packet yet.",
			Details: make(map[string]interface{}),
		})
		return
	}

	a, err := s.gameState.CanChat(player_id, ChatID(chat_msg.Target))

	if err != nil {
		s.Log.Logf("Chat Error: %v", err)
		m.client.send <- protocol.Msg(protocol.ErrorMessage{
			Kind: "ChatChatErr",
			Info: err.Error(),
			Details: map[string]interface{}{
				"target": chat_msg.Target,
			},
		})
		return
	}

	if !a {
		s.Log.Logf("Warning: ChatSend Permission denied: %v", chat_msg.Target)
		m.client.send <- protocol.Msg(protocol.ErrorMessage{
			Kind: "ChatPermDenied",
			Info: "You do not have permissions to send to this chat.",
			Details: map[string]interface{}{
				"target": chat_msg.Target,
			},
		})
		return
	}

	err = s.ChatSend(ChatID(chat_msg.Target), ClientMessage{m.client,
		*protocol.Msg(protocol.ChatSentMessage{
			Sender: s.gameState.Players[player_id].Username,
			Chat:   chat_msg.Chat,
			Origin: chat_msg.Target,
		})})

	if err != nil {
		s.Log.Logf("Warning: ChatSend failed: %v", err)
	}
}

type Chat struct {
	ChatMessages    []protocol.Message
	ControlMessages []protocol.Message
	CurrentAccess   []GroupOrPlayer
}

func (s Server) ChatSend(cid ChatID, m ClientMessage) error {
	ch, ok1 := s.gameState.Chats[cid]
	if !ok1 {
		return errors.New("Cannot find target chat")
	} // Invalid uuid

	ch.ChatMessages = append(ch.ChatMessages, m.msg)

	for con, id := range s.clients {
		// If the user has no uuid, continue
		if id == uuid.Nil || con == m.client {
			continue
		}

		a, err := s.gameState.CanChat(id, cid)
		if err != nil {
			return err
		}

		if a {
			con.send <- &m.msg
		}
	}
	return nil
}

// Checks if the user can chat in a particular chat
func (st *GameState) CanChat(player uuid.UUID, chat ChatID) (bool, error) {
	pstate, ok := st.Players[player]
	if !ok {
		return false, errors.New("Cannot find the given player")
	} // Invalid uuid

	ch, ok1 := st.Chats[chat]
	if !ok1 {
		return false, errors.New("Cannot find target chat")
	} // Invalid uuid

	for _, pog := range ch.CurrentAccess {
		if pog.IsPlayer() {
			if pog.Player_token == player {
				return true, nil
			}
		}

		if pog.Group_name == GIDAny {
			return true, nil
		}

		for _, grp := range pstate.Groups {
			if pog.Group_name == grp {
				return true, nil
			}
		}
	}
	return false, nil
}
