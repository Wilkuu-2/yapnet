package yn_server

import (
	"github.com/gorilla/websocket"
	"time"
	"wilkuu.xyz/yapnet/protocol"
)

type ClientConnection struct {
	server *Server
	conn   *websocket.Conn
	send   chan *protocol.Message
}

type RawClientMessage struct {
	client *ClientConnection
	msg    protocol.RawMessage
}
type ClientMessage struct {
	client *ClientConnection
	msg    protocol.Message
}

const (
	writeTimeout = 120 * time.Second
	pongTimeout  = 10 * time.Second
	pingPeriod   = (pongTimeout * 9) / 10
	maxMessageSz = 1024
)

func (c *ClientConnection) writePump() {
	ping_ticker := time.NewTicker(pingPeriod)
	defer func() {
		ping_ticker.Stop()
		c.conn.Close()
		c.server.disconnect <- c
	}()

	for {
		select {
		case message, ok := <-c.send:
			if !ok {
				c.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}
			err := c.conn.WriteJSON(message)
			if err != nil {
				c.server.Log.Logf("Write error for %v, %v", c.conn.RemoteAddr(), err)
				return
			}
		case <-ping_ticker.C:
			c.conn.SetWriteDeadline(time.Now().Add(writeTimeout))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				c.server.Log.Logf("Ping failed for %v, %v", c.conn.RemoteAddr(), err)
				return
			}
		}
	}
}

func (c *ClientConnection) readPump() {
	defer func() {
		c.conn.Close()
		c.server.disconnect <- c
	}()
	c.conn.SetReadLimit(maxMessageSz)
	c.conn.SetReadDeadline(time.Now().Add(pongTimeout))
	c.conn.SetPongHandler(
		func(string) error {
			c.conn.SetReadDeadline(time.Now().Add(pongTimeout))
			return nil
		})

	for {
		var msg protocol.RawMessage
		err := c.conn.ReadJSON(&msg)
		message := RawClientMessage{
			client: c,
			msg:    msg,
		}
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
			}
			c.server.Log.Logf("Reading Error: %v", err)
			break
		}
		c.server.recieveQueue <- &message
	}
}
