 -- Copyright 2024 Jakub Stachurski
 --
 --  Licensed under the Apache License, Version 2.0 (the "License");
 --  you may not use this file except in compliance with the License.
 --  You may obtain a copy of the License at
 --
 --      http://www.apache.org/licenses/LICENSE-2.0
 --
 --  Unless required by applicable law or agreed to in writing, software
 --  distributed under the License is distributed on an "AS IS" BASIS,
 --  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 --  See the License for the specific language governing permissions and
 --  limitations under the License.


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
