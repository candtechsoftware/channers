use std::sync::{Arc, Condvar, Mutex};
use std::collections::VecDeque; 

struct Shared<T> {
    inner: Mutex<Inner<T>>,
    available: Condvar,

}

struct Inner<T> {
    queue: VecDeque<T>,
    senders: usize,
}



/* Sender Structs
 *
 *
 */

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.message.inner.lock().unwrap();
        inner.senders += 1;
        drop(inner);

        Sender {
            message: Arc::clone(&self.message),
        }
    } 
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut inner = self.message.inner.lock().unwrap();
        inner.senders -= 1;
        let was_last = inner.senders == 0; 
        drop(inner);
        if was_last {
            self.message.available.notify_one();
        }

    }
}


pub struct Sender<T> {
    message: Arc<Shared<T>>,

}

impl<T> Sender<T> {
    pub fn send(&mut self, t: T) {
        let mut inner = self.message.inner.lock().unwrap();
        inner.queue.push_back(t);
        drop(inner);
        self.message.available.notify_one();
    }
}


/* Receiver Structs
 *
 *
 */

pub struct Receiver<T> {
    message: Arc<Shared<T>>,
}


impl<T> Receiver<T> {
    pub fn receive(&mut self) -> Option<T> {
        loop {
            let mut inner = self.message.inner.lock().unwrap();
            match inner.queue.pop_front() {
                Some(t) => return Some(t),
                None if inner.senders == 0 => return None,
                None => {
                    inner = self.message.available.wait(inner).unwrap();
                }
            }
        }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) { 
    let inner = Inner {
        queue: VecDeque::default(),
        senders: 1,
    };
    let shared = Shared{ 
        inner: Mutex::new(inner),
        available: Condvar::new(),
    };
    let shared = Arc::new(shared);
    (
        Sender {
            message: shared.clone()
        },
        Receiver {
            message: shared.clone(),
        }
    )
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn ping_pong() {
        let (mut tx, mut rx) = channel();
        tx.send(42);
        assert_eq!(rx.receive(), Some(42));
    }

    #[test]
    fn closed() {
        let (tx, mut rx) = channel::<()>();
        drop(tx);
        assert_eq!(rx.receive(), None); 
    }
}
