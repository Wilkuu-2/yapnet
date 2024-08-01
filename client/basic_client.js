let ws = null; 
let sendQueue = [] 
let messageLog = []  
let chatbox = document.getElementById("chatbox")
let chatform = document.getElementById("chatform")
let register_form = document.getElementById("register_form")
let login_form = document.getElementById("login_form")

var playerInfo = {
  logged_in: false,
  uuid: "", // TODO: Save in cookies
} 

message_handler = (event) => {
    let message = JSON.parse(event.data) 
    let typ = message.msg_type
    switch (typ){
    case "err":
      console.log("Error: " + message.data["info"] )
      break
    case "char": 
      console.log(message.data)
      let chatMessage = {name: message.data.sender, chat: message.data.chat_content}
      messageLog.push(chatMessage)
      let line = `${chatMessage.name}: ${chatMessage.chat}\n`
      chatbox.innerText += line
      break
    case "welc":
      welcomeHandler(message.data)
      break
    default:
      console.log("Invalid message recieved: " + typ)
      console.log(JSON.stringify(message))
    }
}

open_handler = (event) => {
  console.log("Connected to the server!")
  if(playerInfo.logged_in) { 
    console.log("Logging in again")
    ws.send(JSON.stringify({ 
      msg_type: "back",
      seq: 0, 
      data: { 
        token: playerInfo.uuid ,
        versions: ["1"]
      },})) 
  }
} 

close_handler = (event) => {
  console.log("Disconnected, reconnecting...")
  setTimeout (function() { ws = connect() }, 1000)
}

chatform.onsubmit = (event) => {
  event.preventDefault();
  if (ws != null) {
    chatinput = chatform.querySelector('input[name="chat"]')
    chattext = chatinput.value
    chatinput.value = "" 
    let chatMessage = {name: "You", chat: chattext} 

    messageLog.push(chatMessage) 
    ws.send(JSON.stringify({
      msg_type: "chas",
      seq: 0, 
      data: {
        chat_target: "general",
        chat_content: chattext, 
      },
    }))
    let line = `${chatMessage.name}: ${chatMessage.chat}\n`
    chatbox.innerText += line
  } else {
    console.log("Cannot chat if the connection is not established")
  }
};
login_form.onsubmit = (event) => {
  event.preventDefault();
  console.log("Logging in user")
  if (ws != null) {
    token_field = login_form.querySelector('input[name="token"]')
    token = token_field.value

    ws.send(JSON.stringify({ 
      msg_type: "back",
      seq: 0, 
      data: { 
        token: token ,
        versions: ["1"]
      }, 
    }))  
  } else {
    console.log("Cannot login if the connection is not established")
  }
};

register_form.onsubmit = (event) => {
  event.preventDefault();
  console.log("Registering user")
  if (ws != null) {
    username_field = register_form.querySelector('input[name="username"]')
    username = username_field.value

    ws.send(JSON.stringify({ 
      msg_type: "helo",
      seq: 0, 
      data: { 
        username: username,
        versions: ["1"]
      }, 
    }))  
  } else {
    console.log("Cannot login if the connection is not established")
  }
};

function welcomeHandler(message) {
    console.log("Got a welcome message!!")
    username_field = register_form.querySelector('input[name="username"]')
    token_field = login_form.querySelector('input[name="token"]')

    reg_button = register_form.querySelector('input[type="submit"]')
    log_button = login_form.querySelector('input[type="submit"]')

    playerInfo.logged_in = true
    playerInfo.uuid = message.token
    playerInfo.username = message.name

    username_field.value = message.name
    username_field.readOnly = true 

    token_field.value = message.token
    token_field.readOnly = true 
    
    reg_button.disabled = true
    log_button.disabled = true
    
    
    
} 

function connect()
{
  // Todo make this a button instead
  var nws = new WebSocket("ws://localhost:8080/ws")
  nws.onmessage = message_handler
  nws.onopen = open_handler
  nws.onclose = close_handler
  return nws
}

ws = connect()
