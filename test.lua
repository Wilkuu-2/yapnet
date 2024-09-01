print("Lua is being evaluated")

Define_chats = function (n)
    local chats = {}
    for i = 1, n, 1 do
       local name = "general" .. i
       yapi.yn_api_test(name);
       chats[name] = { allowed = "any" }
    end
    return chats
end

return {
  chats = Define_chats(15),
  on_chat = function (frame, t, n, c)
    frame:send_message ({
      msg_type = "chat",
      seq = 0,
      data = {
        chat_sender = "SYSTEM",
        chat_target = t,
        chat_content = "Hello" .. n .. "You said:\n" .. c,
      }
    });

  end
}
