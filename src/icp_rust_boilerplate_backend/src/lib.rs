#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use ic_cdk::api;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;


enum Category {
    Programming,
    Health,
    LifeStyle,
}

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Course {
    id: u64,
    creator_name: String,
    creator_address: String,
    title: String,
    body: String,
    attachment_url: String,
    keyword: String,
    created_at: u64,
    updated_at: Option<u64>,
}

// a trait that must be implemented for a struct that is stored in a stable struct
impl Storable for Course {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }
}

// another trait that must be implemented for a struct that is stored in a stable struct
impl BoundedStorable for Course {
    const MAX_SIZE: u32 = 1024;
    const IS_FIXED_SIZE: bool = false;
}

thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> = RefCell::new(
        MemoryManager::init(DefaultMemoryImpl::default())
    );

    static ID_COUNTER: RefCell<IdCell> = RefCell::new(
        IdCell::init(MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(0))), 0)
            .expect("Cannot create a counter")
    );

    static STORAGE: RefCell<StableBTreeMap<u64, Course, Memory>> =
        RefCell::new(StableBTreeMap::init(
            MEMORY_MANAGER.with(|m| m.borrow().get(MemoryId::new(1)))
    ));
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct CoursePayLoad {
    title: String,
    creator_name: String,
    body: String,
    attachment_url: String,
    keyword: String,
}

#[ic_cdk::query]
fn get_message(id: u64) -> Result<Course, Error> {
    match _get_message(&id) {
        Some(message) => Ok(message),
        None => Err(Error::NotFound {
            msg: format!("a message with id={} not found", id),
        }),
    }
}

#[ic_cdk::update]
fn add_message(message: CoursePayLoad) -> Option<Course> {
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");

    let puffy: String = api::caller().to_string();
    let message = Course {
        id,
        creator_address: puffy,
        creator_name: message.creator_name,
        title: message.title,
        body: message.body,
        attachment_url: message.attachment_url,
        created_at: time(),
        updated_at: None,
        keyword: message.keyword
    };

    do_insert(&message);
    Some(message)
}

#[ic_cdk::update]
fn update_message(id: u64, payload: CoursePayLoad) -> Result<Course, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut message) => {
            let caller = api::caller();
            if message.creator_address != caller.to_string() {
                Err(Error::Unauthorized {
                    msg: format!(
                        "you are not the creator of id={}",
                        id
                    ),
                })
            } else {
                message.attachment_url = payload.attachment_url;
                message.body = payload.body;
                message.title = payload.title;
                message.updated_at = Some(time());
                do_insert(&message);
                Ok(message)
            }
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update a message with id={}. message not found",
                id
            ),
        }),
    }
}

// helper method to perform insert.
fn do_insert(message: &Course) {
    STORAGE.with(|service| service.borrow_mut().insert(message.id, message.clone()));
}

#[ic_cdk::update]
fn delete_message(id: u64) -> Result<Course, Error> {
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

#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    Unauthorized { msg: String }
}

// a helper method to get a message by id. used in get_message/update_message
fn _get_message(id: &u64) -> Option<Course> {
    STORAGE.with(|service| service.borrow().get(id))
}

// need this to generate candid
ic_cdk::export_candid!();