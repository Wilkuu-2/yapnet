use crate::protocol::message::*; 

pub struct History {
    inner: Vec<Message>,
    seq: u64,
}

impl History {
    pub fn new() -> Self {
        History {
            inner: vec![],
            seq: 0,
        }
    }

    pub fn iter<'a>(&'a self) -> HistoryIter<'a> {
        HistoryIter {
            index: 0, 
            data: self,
        }
        
    }

    /// This should be only done when sending messages.
    pub fn state_message(&mut self, m: MessageData) -> &Message {
        let s = self.seq;
        self.seq += 1;
        let message = Message { seq: s, data: m };
        self.inner.push(message);
        self.inner.last().expect("Just pushed")
    }

    pub fn push_and_serialize(&mut self, m: MessageData) -> String {
        let s = self.seq;
        self.seq += 1;
        let message = Message { seq: s, data: m };
        let d = serde_json::to_string(&message).unwrap();
        // TODO: Handle error
        self.inner.push(message);
        d
    }

    pub fn push(&mut self, m:MessageData){
        let s = self.seq;
        self.seq += 1;
        let message = Message { seq: s, data: m };
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
    type Item = &'a Message;
    fn next(&mut self) -> Option<Self::Item> {
        let o = self.data.inner.get(self.index);
        self.index += 1; 
        return o; 
        
    } 

        
}

