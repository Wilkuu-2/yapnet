// Copyright 2024 Jakub Stachurski
//
//   Licensed under the Apache License, Version 2.0 (the "License");
//   you may not use this file except in compliance with the License.
//   You may obtain a copy of the License at
//
//       http://www.apache.org/licenses/LICENSE-2.0
//
//   Unless required by applicable law or agreed to in writing, software
//   distributed under the License is distributed on an "AS IS" BASIS,
//   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//   See the License for the specific language governing permissions and
//   limitations under the License.

use crate::protocol::{body::MessageV2Enum, MessageV2};

pub struct History {
    inner: Vec<MessageV2>,
    seq: u64,
}

impl Default for History {
    fn default() -> Self {
        Self::new()
    }
}

impl History {
    pub fn new() -> Self {
        History {
            inner: vec![],
            seq: 0,
        }
    }

    pub fn iter(&self) -> HistoryIter<'_> {
        HistoryIter {
            index: 0,
            data: self,
        }
    }

    /// This should be only done when sending messages.
    pub fn state_message(&mut self, m: MessageV2Enum) -> &MessageV2 {
        let s = self.seq;
        self.seq += 1;
        let message = MessageV2 { seq: s, data: m };
        self.inner.push(message);
        self.inner.last().expect("Just pushed")
    }

    pub fn push_and_serialize(&mut self, m: MessageV2Enum) -> String {
        let s = self.seq;
        self.seq += 1;
        let message = MessageV2 { seq: s, data: m };
        let d = serde_json::to_string(&message).unwrap();
        // TODO: Handle error
        self.inner.push(message);
        d
    }

    pub fn push(&mut self, m: MessageV2Enum) {
        let s = self.seq;
        self.seq += 1;
        let message = MessageV2 { seq: s, data: m };
        self.inner.push(message);
    }

    // TODO: Move to server code

    pub fn print_state(&self) {
        println!("---- [State] ----");
        for m in self.inner.iter() {
            println!("[{}] {:?}", m.seq, m.data)
        }
    }
}

pub struct HistoryIter<'a> {
    data: &'a History,
    index: usize,
}

impl<'a> Iterator for HistoryIter<'a> {
    type Item = &'a MessageV2;
    fn next(&mut self) -> Option<Self::Item> {
        let o = self.data.inner.get(self.index);
        self.index += 1;
        o
    }
}
