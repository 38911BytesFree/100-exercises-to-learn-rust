// TODO: Convert the implementation to use bounded channels.
use crate::data::{Ticket, TicketDraft};
use crate::store::{TicketId, TicketStore};
use std::sync::mpsc::{sync_channel, Receiver, RecvError, SyncSender, TrySendError};

pub mod data;
pub mod store;

#[derive(Clone)]
pub struct TicketStoreClient {
    sender: SyncSender<Command>
}

#[derive(Debug)]
pub enum InsertError {
    TrySendError(String),
    RecvError(RecvError),
}

impl TicketStoreClient {
    pub fn insert(&self, draft: TicketDraft) -> Result<TicketId, InsertError> {
        let (sender, receiver) = sync_channel(10);
        let cmd = Command::Insert {
            draft,
            response_channel: sender
        };
        match self.sender.try_send(cmd) {
            Ok(_) => {
                match receiver.recv() {
                    Ok(ticket_id) => Ok(ticket_id),
                    Err(recv_error) => Err(InsertError::RecvError(recv_error)),
                }
            }
            Err(_send_error) => Err(InsertError::TrySendError("send_error".to_string())),
        }   
     }

    pub fn get(&self, id: TicketId) -> Result<Option<Ticket>, String> {
        let (sender, receiver) = sync_channel(10);
        let cmd = Command::Get {
            id,
            response_channel: sender
        };
        self.sender.try_send(cmd).unwrap();
        Ok(Some(receiver.recv().unwrap()))
    }
}

pub fn launch(capacity: usize) -> TicketStoreClient {
    let (sender, receiver) = sync_channel(capacity);
    std::thread::spawn(move || server(receiver));
    TicketStoreClient {
        sender
    }
}

enum Command {
    Insert {
        draft: TicketDraft,
        response_channel: SyncSender<TicketId>,
    },
    Get {
        id: TicketId,
        response_channel: SyncSender<Ticket>,
    },
}

pub fn server(receiver: Receiver<Command>) {
    let mut store = TicketStore::new();
    loop {
        match receiver.recv() {
            Ok(Command::Insert {
                draft,
                response_channel,
            }) => {
                let id = store.add_ticket(draft);
                response_channel.send(id).unwrap()
            }
            Ok(Command::Get {
                id,
                response_channel,
            }) => {
                let ticket = store.get(id);
                response_channel.send(ticket.cloned().unwrap()).unwrap()
            }
            Err(_) => {
                // There are no more senders, so we can safely break
                // and shut down the server.
                break;
            }
        }
    }
}
