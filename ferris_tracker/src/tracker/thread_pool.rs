use log::info;
use std::{
    error::Error,
    fmt,
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

// ========================================================================================

#[derive(Debug)]
pub enum ThreadPoolError {
    SendingJobToWorkerError(String),
}

impl fmt::Display for ThreadPoolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\n    {:#?}\n", self)
    }
}

impl Error for ThreadPoolError {}

// ========================================================================================

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
                        Err(_) => break,
                    }
                }
                Err(_) => break,
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

    pub fn execute<F>(&self, f: F) -> Result<(), ThreadPoolError>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender
            .send(Message::NewJob(job))
            .map_err(|err| ThreadPoolError::SendingJobToWorkerError(err.to_string()))?;
        Ok(())
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
