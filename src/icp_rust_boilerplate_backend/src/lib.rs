#[macro_use]
// Rust data structures
extern crate serde;

// Interafce of canister
use candid::{Decode, Encode};
// Canister development kit -> interact with the ICP network
use ic_cdk::api::time;
// Stable data structures across updateds
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
// Standard library of rust
use std::{borrow::Cow, cell::RefCell};

// Store state of canister
type Memory = VirtualMemory<DefaultMemoryImpl>;
// Generatge unique id
type IdCell = Cell<u64, Memory>;

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Message {
    id: u64,
    title: String,
    body: String,
    attachment_url: String,
    created_at: u64,
    updated_at: Option<u64>,
}

// Convert message(struct) into byte and byte into message
// impl is the keyword used as implementation of a trait
// Trait is like an interface
impl Storable for Message {

    // Self means the type that the function is present inside -> most likely Message
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// maximum length of the struct and whether it is fixed or not
impl BoundedStorable for Message {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

// Store the canister state which can be accessed from any part of the code
// All these are just variables which can be called from any thread
thread_local! {

    // Store canisters virtual memory so that it can be accessed from anywhere in the code
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    // ID  count of the canister
    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    // Canister's storage
    static STORAGE: RefCell<StableBTreeMap<u64, Message, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

// Input from the user for a new entry
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct MessagePayload {
    title: String,
    body: String,
    attachment_url: String,
}

// Custom error which we use when the data is not found
#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
}



// Obtain message from canister
// First line indicates that this method can be called from the outside as a query
#[ic_cdk::query]
fn get_message(id: u64) -> Result<Message, Error> {
    // match is similar to switch case
    match _get_message(&id) {
        //Returns the message
        Some(message) => Ok(message),
        None => Err(Error::NotFound {
            msg: format!("Message with id={} not found", id),
        }),
    }
}

// Actual function which retreives the message from the thread storage
// s is the reference of the canister storage
fn _get_message(id: &u64) -> Option<Message> {
    STORAGE.with(|s| s.borrow().get(id))
}

#[ic_cdk::update]
fn add_message(message: MessagePayload) -> Option<Message> {

    // obtain the counter value and increment it
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");
    let message = Message {
        id,
        title: message.title,
        body: message.body,
        attachment_url: message.attachment_url,
        created_at: time(),
        updated_at: None,
    };
    do_insert(&message);
    Some(message)
}

// Performs the actual insert into the storage
fn do_insert(message: &Message) {
    STORAGE.with(|service| service.borrow_mut().insert(message.id, message.clone()));
}

#[ic_cdk::update]
fn update_message(id: u64, payload: MessagePayload) -> Result<Message, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut message) => {
            message.attachment_url = payload.attachment_url;
            message.body = payload.body;
            message.title = payload.title;
            message.updated_at = Some(time());
            do_insert(&message);
            Ok(message)
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update a message with id={}. message not found",
                id
            ),
        }),
    }
}

#[ic_cdk::update]
fn delete_message(id: u64) -> Result<Message, Error> {
    match STORAGE.with(|service| service.borrow_mut().remove(&id)) {
        Some(message) => Ok(message),
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete a message with id={}. message not found.",
                id
            ),
        }),
    }
}

// need this to generate candid
ic_cdk::export_candid!();