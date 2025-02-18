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

use std::usize;

use crate::protocol::{body::MessageV2Enum, MessageV2};

#[derive(Debug)]
pub struct History {
    inner: Vec<MessageV2>,
    start: u64,
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
            start: 0, 
            seq: 0,
        }
    }

    pub fn iter(&self) -> HistoryIter<'_> {
        HistoryIter {
            index: 0,
            data: self,
        }
    }

    pub fn get_frame(&self) -> Self{
        let mut out = Self::new();
        out.seq = self.seq; 
        out.start = self.seq;
        out 
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

    pub fn print_state(&self) {
        println!("---- [State] ----");
        for m in self.inner.iter() {
            println!("[{}] {:?}", m.seq, m.data)
        }
    }
    pub fn get_message(&self ,seq: u64) -> Option<&MessageV2>{
        if seq < self.start || seq >= self.seq {
            return None; 
        }
        self.inner.get((seq - self.start) as usize)
    }

    pub fn remove_message(&mut self, seq: u64) -> bool {
        if seq < self.start || seq >= self.seq {
            return false; 
        }
        let i = seq - self.start;
        for x in self.inner.iter_mut().skip(i as usize) {
            x.seq -= 1; 
        }

        self.inner.remove(seq as usize);
        true
    }

    pub fn merge(&mut self, to_merge: History){
        assert_eq!(self.seq, to_merge.start); 
        self.inner.extend(to_merge.inner); 
        self.seq = to_merge.seq;
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
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
