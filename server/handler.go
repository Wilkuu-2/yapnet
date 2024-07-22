package yn_server

import (
	"fmt"

	"github.com/google/uuid"
	"wilkuu.xyz/yapnet_v1/protocol"
)

func (s Server) handleMessage(m *ClientMessage) {
	switch m.msg.Msg_type {
	case protocol.MsgTypeEcho:
		{
			s.handleEcho(m)
		}
	case protocol.MsgTypeHello:
		{
			s.handleHello(m)
		}
	case protocol.MsgTypeBack:
		{
			s.handleBack(m)
		}
	case protocol.MsgTypeChatSend:
		{
			s.handleChat(m)
		}
	default:
		{
			s.handleInvalid(m)
		}
	}

}
func (s Server) handleEcho(m *ClientMessage) {
	s.Log.Logf("Handling an echo from %v!", m.client.conn.RemoteAddr())
	m.client.send <- &m.msg
}

func (s Server) handleHello(m *ClientMessage) {
	uuid := uuid.New()
	hello := m.msg.Data.(protocol.HelloMessage)
	s.Log.Logf("Handling an hello from %v!, name: %v", m.client.conn.RemoteAddr(),hello.Name)
	
	
	if err := s.gameState.addPlayer(uuid,hello.Name); err != nil {
		m.client.send <- protocol.Msg(protocol.ErrorMessage{
				Kind:    "NonUniqueUsername",
				Info:    err.Error(),
				Details: map[string]interface{}{},
		})
		return
	}
	s.clients[m.client] = uuid
        
	m.client.send <- protocol.Msg(protocol.WelcomeMessage{
		Name: hello.Name,
		Token: uuid, 	
		Version: "1",
	})
   
}

func (s Server) handleBack(m *ClientMessage) {
	hello := m.msg.Data.(protocol.BackMessage)
	s.Log.Logf("Handling an back from %v!, uuid: %v", m.client.conn.RemoteAddr(),hello.Token)
	
	for _, token := range s.clients {
		if token == hello.Token {
			m.client.send <- protocol.Msg(protocol.ErrorMessage{
					Kind:    "AlreadyConnected",
					Info:    "You are already connected to the server",
					Details: map[string]interface{}{},
			})
			return
		}	
	} 

	player, ok := s.gameState.Players[hello.Token] 

	if !ok{
		m.client.send <- protocol.Msg(protocol.ErrorMessage{
				Kind:    "InvalidToken",
				Info:    "The token given is not a valid token in this game",
				Details: map[string]interface{}{},
		})
		return
	} 

	s.clients[m.client] = hello.Token 
        
	m.client.send <- protocol.Msg(protocol.WelcomeMessage{
		Name: player.Username,
		Token: hello.Token, 	
		Version: "1",
	})
   
}

func (s Server) handleInvalid(m *ClientMessage) {
	s.Log.Logf("Handling an invalid message '%v' from %v!",m.msg.Msg_type, m.client.conn.RemoteAddr())
	m.client.send <- protocol.Msg(protocol.ErrorMessage{
			Kind:    "InvalidMSGType",
			Info:    fmt.Sprintf("The message type %v is not a valid type.", m.msg.Msg_type),
			Details: map[string]interface{}{"invalid_type": string(m.msg.Msg_type)},
	})
}
