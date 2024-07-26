package yn_server

import (
	"net/http"

	"github.com/google/uuid"
	"github.com/gorilla/websocket"
	"wilkuu.xyz/yapnet/protocol"
)

var upgrader = websocket.Upgrader{
	WriteBufferSize: 1024,
	ReadBufferSize:  1024,
	CheckOrigin:     func(r *http.Request) bool { return true }, // Allow all origins
}

type ClientState struct {
	online   bool
	username string
}

type Server struct {
	Log          Logger
	recieveQueue chan *RawClientMessage
	clients      map[*ClientConnection]uuid.UUID
	connect      chan *ClientConnection
	disconnect   chan *ClientConnection
	gameState    GameState
}
type Logger struct {
	Logf func(string, ...interface{})
}

func (s *Server) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	s.Log.Logf("Recieved connection from %v", r.RemoteAddr)
	c, err := upgrader.Upgrade(w, r, nil)

	if err != nil {
		s.Log.Logf("Failed to upgrade connection for %v\nCause: %v", r.RemoteAddr, err)
		return
	}

	ccon := &ClientConnection{
		server: s,
		conn:   c,
		send:   make(chan *protocol.Message),
	}

	s.connect <- ccon

	go ccon.writePump()
	go ccon.readPump()
}

func NewServer(logf func(string, ...interface{})) *Server {
	return &Server{
		Log:          Logger{Logf: logf},
		recieveQueue: make(chan *RawClientMessage),
		clients:      make(map[*ClientConnection]uuid.UUID),
		connect:      make(chan *ClientConnection),
		disconnect:   make(chan *ClientConnection),
		gameState:    newState(),
	}
}

func (s *Server) Run() {
	s.Log.Logf("Running the server!")
	for {
		select {
		case client := <-s.connect:
			s.Log.Logf("Adding a client: %v!", client.conn.RemoteAddr())
			s.clients[client] = uuid.Nil

		case client := <-s.disconnect:
			if _, ok := s.clients[client]; ok {
				s.Log.Logf("Removing client!")
				s.disconnectPlayer(client)
				delete(s.clients, client)
				close(client.send)
				s.PrintPlayers()
			}
		case message := <-s.recieveQueue:
			s.Log.Logf("Recieved a message from client: %v", message.client.conn.RemoteAddr())
			s.handleMessage(message)
		}
	}
}

// Broadcasts given ClientMessage to all other clients
func (s *Server) ClientBroadcast(m ClientMessage) {
	for client := range s.clients {
		if client != m.client {
			client.send <- &m.msg
		}
	}
}

func CheckMark(checked bool) string {
	if checked {
		return "[x]"
	}
	return "[ ]"
}

func (s *Server) PrintPlayers() {
	s.Log.Logf("-- Clients: %d | Players %d --", len(s.clients), len(s.gameState.Players))
	for uuid, player := range s.gameState.Players {
		s.Log.Logf("%v %v: %v",
			CheckMark(player.Online),
			player.Username,
			uuid,
		)
	}
}

func (s *Server) disconnectPlayer(c *ClientConnection) {
	player, ok := s.gameState.Players[s.clients[c]]

	if !ok {
		return
	}

	player.Online = false

}
