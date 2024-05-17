#[macro_use]
extern crate serde;
use candid::{Decode, Encode};
use ic_cdk::api::time;
use std::sync::Mutex;
use ic_cdk::api;
use ic_stable_structures::memory_manager::{MemoryId, MemoryManager, VirtualMemory};
use ic_stable_structures::{BoundedStorable, Cell, DefaultMemoryImpl, StableBTreeMap, Storable};
use std::{borrow::Cow, cell::RefCell};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type IdCell = Cell<u64, Memory>;

// use ic_cdk::api::wallet;
// use ic_cdk::export::candid::{CandidType, Principal};
// use ic_cdk::storage;



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
    category: String,
    created_at: u64,
    updated_at: Option<u64>,
    contact: String,
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

    static ADMIN_ADDRESS: Mutex<Option<String>> = Mutex::new(None);
    static MODERATOR_ADDRESSES: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct CoursePayLoad {
    title: String,
    creator_name: String,
    body: String,
    attachment_url: String,
    keyword: String,
    category: String,
    contact: String,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct CourseUpdatePayLoad {
    title: Option<String>,
    creator_name: Option<String>,
    body: Option<String>,
    attachment_url: Option<String>,
    keyword: Option<String>,
    category: Option<String>,
    contact: Option<String>,
}

#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct FilterPayLoad {
    keyword: Option<String>,
    category: Option<String>,
    creator_address: Option<String>,
}

// Small confusion in what the admin address is not set is doing
#[ic_cdk::update]
fn set_admin_address(address: String) -> Result<(), String> {
    ADMIN_ADDRESS.with(|admin_address| {
        let caller = api::caller().to_string();
        let mut admin_address = admin_address.lock().unwrap();
        if admin_address.is_none() {
            *admin_address = Some(address);
            Ok(())
        } else if let Some(admin) = &*admin_address {
            if caller == *admin {
                *admin_address = Some(address);
                Ok(())
            } else {
                Err("Only admin can change the admin address".to_string())
            }
        } else {
            Err("Admin address is not set".to_string())
        }
    })
}

#[ic_cdk::update]
fn add_moderator_address(address: String) -> Result<(), String> {
    ADMIN_ADDRESS.with(|admin_address| {
        if let Some(admin) = &*admin_address.lock().unwrap() {
            let caller = api::caller().to_string();
            if caller == *admin {
                MODERATOR_ADDRESSES.with(|moderator_addresses| {
                    moderator_addresses.lock().unwrap().push(address);
                });
                Ok(())
            } else {
                Err("Only admin can add moderators".to_string())
            }
        } else {
            Err("Admin address is not set".to_string())
        }
    })
}



// #[derive(candid::CandidType, Serialize, Deserialize, Default)]
// struct DonationPayload {
//     amount: u64,
//     recipient: Principal, 
// }


#[ic_cdk::query]
fn get_course(id: u64) -> Result<Course, Error> {
    match _get_course_(&id) {
        Some(course) => Ok(course),
        None => Err(Error::NotFound {
            msg: format!("a course with id={} not found", id),
        }),
    }
}

// Internal function
fn _get_course_(id: &u64) -> Option<Course> {
    STORAGE.with(|service| service.borrow().get(id))
}

#[ic_cdk::query]
fn filter_courses_and(payload: FilterPayLoad) -> Result<Vec<Course>, Error> {
    let courses: Vec<Course> = STORAGE.with(|storage| {
        storage.borrow().iter()
            .filter_map(|(_, course)| {
                let mut matches = true;
                if let Some(ref keyword) = payload.keyword {
                    matches &= course.keyword == *keyword;
                }
                if let Some(ref category) = payload.category {
                    matches &= course.category == *category;
                }
                if let Some(ref creator_address) = payload.creator_address {
                    matches &= course.creator_address == *creator_address;
                }
                if matches {
                    Some(course.clone())
                } else {
                    None
                }
            })
            .collect()
    });

    if courses.is_empty() {
        Err(Error::NotFound{
            msg: (
                "couldn't find a course with provided inputs".to_string()
            ),
        })
    } else {
        Ok(courses)
    }
}

#[ic_cdk::query]
fn filter_courses_or(payload: FilterPayLoad) -> Result<Vec<Course>, Error> {
    let courses: Vec<Course> = STORAGE.with(|storage| {
        storage.borrow().iter()
            .filter_map(|(_, course)| {
                let mut matches = false;
                if let Some(ref keyword) = payload.keyword {
                    matches |= course.keyword == *keyword; 
                }
                if let Some(ref category) = payload.category {
                    matches |= course.category == *category; 
                }
                if let Some(ref creator_address) = payload.creator_address {
                    matches |= course.creator_address == *creator_address; 
                }
                if matches {
                    Some(course.clone())
                } else {
                    None
                }
            })
            .collect()
    });

    if courses.is_empty() {
        Err(Error::NotFound{
            msg: (
                "couldn't find a course with provided inputs".to_string()
            ),
        })
    } else {
        Ok(courses)
    }
}


#[ic_cdk::update]
fn add_course(course: CoursePayLoad) -> Result<Course, Error> {
    //Validation Logic
    if course.title.is_empty()
    || course.creator_name.is_empty()
    || course.body.is_empty()
    || course.attachment_url.is_empty()
    || course.keyword.is_empty()
    || course.category.is_empty()
    || course.contact.is_empty()
    {
        return Err(Error::EmptyFields {
            msg: "Please fill in all the required fields to create a course".to_string(),
        });
    }
    let id = ID_COUNTER
        .with(|counter| {
            let current_value = *counter.borrow().get();
            counter.borrow_mut().set(current_value + 1)
        })
        .expect("cannot increment id counter");

    let address_string: String = api::caller().to_string();
    let course = Course {
        id,
        creator_address: address_string,
        creator_name: course.creator_name,
        title: course.title,
        body: course.body,
        attachment_url: course.attachment_url,
        created_at: time(),
        updated_at: None,
        category: course.category,
        keyword: course.keyword,
        contact: course.contact
    };

    do_insert(&course);
    Ok(course)
}

#[ic_cdk::update]
fn update_course(id: u64, payload: CourseUpdatePayLoad) -> Result<Course, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut course) => {
            let caller = api::caller();
            if course.creator_address != caller.to_string() {
                Err(Error::UnAuthorized {
                    msg: format!(
                        "you are not the creator of id={}",
                        id
                    ),
                })
            } else {
                if let Some(title) = payload.title {
                    course.title = title;
                }
                if let Some(creator_name) = payload.creator_name {
                    course.creator_name = creator_name;
                }
                if let Some(body) = payload.body {
                    course.body = body;
                }
                if let Some(attachment_url) = payload.attachment_url {
                    course.attachment_url = attachment_url;
                }
                if let Some(keyword) = payload.keyword {
                    course.keyword = keyword;
                }
                if let Some(category) = payload.category {
                    course.category = category;
                }
                if let Some(contact) = payload.contact {
                    course.contact = contact;
                }
                course.updated_at = Some(time());
                do_insert(&course);
                Ok(course)
            }
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't update a course with id={}. course not found",
                id
            ),
        }),
    }
}

// helper method to perform insert.
fn do_insert(course: &Course) {
    STORAGE.with(|service| service.borrow_mut().insert(course.id, course.clone()));
}

#[ic_cdk::update]
fn delete_course(id: u64) -> Result<Course, Error> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(course) => {
            let caller = api::caller();
            if course.creator_address != caller.to_string() {
                Err(Error::UnAuthorized {
                    msg: format!(
                        "you are not the creator of the course id={}",
                        id
                    ),
                })
            } else {
                STORAGE.with(|service| service.borrow_mut().remove(&id));
                Ok(course)
            }
        }
        None => Err(Error::NotFound {
            msg: format!(
                "couldn't delete course with id={}. course not found.",
                id
            ),
        }),
    }
}

#[ic_cdk::update]
fn delete_all_courses() -> Result<Vec<Course>, Error> {
    let caller = api::caller().to_string(); // Convert caller address to string
    let mut deleted_courses: Vec<Course> = Vec::new(); // Keep track of deleted courses

    STORAGE.with(|service| {
        let mut storage = service.borrow_mut();
        let mut keys_to_remove = Vec::new(); // Keep track of keys to remove

        // Iterate through storage to find and remove matching courses
        for (id, course) in storage.iter() {
            if course.creator_address == caller {
                // If creator address matches caller, mark for removal
                keys_to_remove.push(id.clone());
                deleted_courses.push(course.clone()); // Add course to deleted list
            }
        }

        // Remove courses from storage
        for key in keys_to_remove {
            storage.remove(&key);
        }
    });

    if deleted_courses.is_empty() {
        Err(Error::NotFound {
            msg: "No courses found for the caller. Nothing to delete.".to_string(),
        })
    } else {
        Ok(deleted_courses)
    }
}

// // Function to donate ICP tokens
// pub fn donate_icp(payload: DonationPayload) -> Result<(), String> {
//     // Get the authenticated user's wallet address
//     let caller = wallet::get_sender();
//     let sender_wallet = caller.expect("Failed to get sender wallet");

//     // Check if the user has sufficient balance
//     let sender_balance = wallet::balance(sender_wallet)
//         .map_err(|err| format!("Failed to get sender balance: {:?}", err))?;
//     if sender_balance < payload.amount {
//         return Err("Insufficient balance".to_string());
//     }

//     // Transfer ICP tokens to the recipient
//     let transfer_result = wallet::transfer_to_account(payload.recipient, payload.amount);
//     if let Err(err) = transfer_result {
//         return Err(format!("Failed to transfer ICP: {:?}", err));
//     }

//     Ok(())
// }



#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    UnAuthorized { msg: String },
    EmptyFields {msg: String}
}

// need this to generate candid
ic_cdk::export_candid!();