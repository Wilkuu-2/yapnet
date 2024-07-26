package protocol

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
	MsgTypeChatRecap MsgType = "recp"
	// Misc
	MsgTypeEcho MsgType = "echo"
	MsgTypeRaw  MsgType = "raw"
)
