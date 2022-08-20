use log::info;
use std::{
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Message>>>) -> Worker {
        let thread = Some(thread::spawn(move || loop {
            let lock = receiver.lock();
            match lock {
                Ok(receiver) => {
                    let message = receiver.recv();
                    match message {
                        Ok(Message::NewJob(job)) => {
                            info!("Worker {} got a job; executing.", id);
                            job();
                        }
                        Ok(Message::Terminate) => {
                            info!("Worker {} was told to terminate.", id);
                            break;
                        }
                        Err(_) => break, //Ver como informar este error
                    }
                }
                Err(_) => break, //Ver como informar este error
            }
        }));
        Worker { id, thread }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)))
        }

        ThreadPool { workers, sender }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        let _ = self.sender.send(Message::NewJob(job));
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            let _ = self.sender.send(Message::Terminate);
        }

        info!("Shutting down all workers");

        for worker in &mut self.workers {
            info!("Shutting down worker {}", worker.id);
            if let Some(thread) = worker.thread.take() {
                let _ = thread.join();
            }
        }
    }
}
