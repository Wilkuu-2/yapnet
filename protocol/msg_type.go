package protocol

// The type of the message, it defines what type the inner data
type MsgType string
const (
	// Errors
	MsgTypeInvalid MsgType = "invd"
	MsgTypeError   MsgType = "err"
	// Introduction Protocol
	MsgTypeHello   MsgType = "helo"
	MsgTypeBack    MsgType = "back"
	MsgTypeWelcome MsgType = "welc"
	// Chat protocol
	MsgTypeChatSend  MsgType = "chas"
	MsgTypeChatSent  MsgType = "char"
	// Syncing
	MsgTypeRecapHead    MsgType = "rech"
	MsgTypeRecapTail     MsgType = "recx"
	// Misc
	MsgTypeEcho MsgType = "echo"
	MsgTypeRaw  MsgType = "raw"
	MsgTypeEmpty MsgType = "" 
)
