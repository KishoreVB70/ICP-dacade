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

#[derive(candid::CandidType, Clone, Serialize, Deserialize, Default)]
struct Course {
    id: u64,
    creator_name: String,
    creator_address: String, // Stores the principal of the caller in string format
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

    // Stores a single admin address
    static ADMIN_ADDRESS: Mutex<Option<String>> = Mutex::new(None);

    // Stores the moderator addresses
    static MODERATOR_ADDRESSES: Mutex<Vec<String>> = Mutex::new(Vec::new());

    // Stores the addresses of banned users
    static BANNED_ADDRESSES: Mutex<Vec<String>> = Mutex::new(Vec::new());
}

// Payload to add a new course obtained from the user
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

// Payload to update a course obtained from the user
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

// Payload to filter all the available courses
#[derive(candid::CandidType, Serialize, Deserialize, Default)]
struct FilterPayLoad {
    keyword: Option<String>,
    category: Option<String>,
    creator_address: Option<String>,
    start_date: Option<u64>,
    end_date: Option<u64>,
}

// Function to set the admin
#[ic_cdk::update]
fn set_admin_address(address: String) -> Result<(), String> {
    let caller: String = api::caller().to_string();
    ADMIN_ADDRESS.with(|admin_address| {
        let mut admin = admin_address.lock().unwrap();

        if admin.is_none() || admin.as_ref().unwrap() == &caller {
            *admin = Some(address);
            Ok(())
        } else {
            Err("Only admin can change the admin address".to_string())
        }
    })
}

// Adds a moderator. Only the admin can add moderators.
#[ic_cdk::update]
fn add_moderator(address: String) -> Result<(), String> {
    let caller = api::caller().to_string();

    if _is_admin(caller) {
        MODERATOR_ADDRESSES.with(|moderator_addresses| {
            let mut addresses = moderator_addresses.lock().unwrap();
            
            if addresses.len() >= 5 {
                Err("Maximum number of moderators reached".to_string())
            } else if addresses.contains(&address) {
                Err("Moderator address already exists".to_string())
            } else {
                addresses.push(address);
                Ok(())
            }
        })
    } else {
        Err("Only admin can add moderators".to_string())
    }
}

// Removes a moderator. Only admin can remove moderators.
#[ic_cdk::update]
fn remove_moderator(address: String) -> Result<(), String> {
    let caller = api::caller().to_string();

    if _is_admin(caller) {
        MODERATOR_ADDRESSES.with(|moderator_addresses| {
            let mut addresses = moderator_addresses.lock().unwrap();
            if addresses.contains(&address) {
                addresses.retain(|a| a != &address);
                Ok(())
            } else {
                Err("Provided address is not a moderator".to_string())
            }
        })
    } else {
        Err("Only admin can remove moderators".to_string())
    }
}

// Retrieves a course based on its ID.
#[ic_cdk::query]
fn get_course(id: u64) -> Result<Course, String> {
    match _get_course_(&id) {
        Some(course) => Ok(course),
        None => Err(format!("A course with id={} not found", id)),
    }
}

// Filters courses based on the provided criteria (AND condition)
#[ic_cdk::query]
fn filter_courses_and(payload: FilterPayLoad) -> Result<Vec<Course>, String> {
    if payload.keyword.is_none() && payload.category.is_none() && payload.creator_address.is_none() && payload.start_date.is_none() && payload.end_date.is_none() {
        return Err("Filter payload is empty; at least one filter criterion must be provided".to_string());
    }

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
                if let Some(start_date) = payload.start_date {
                    matches &= course.created_at >= start_date;
                }
                if let Some(end_date) = payload.end_date {
                    matches &= course.created_at <= end_date;
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
        Err("Couldn't find a course with provided inputs".to_string())
    } else {
        Ok(courses)
    }
}

// Filters courses based on the provided criteria (OR condition).
#[ic_cdk::query]
fn filter_courses_or(payload: FilterPayLoad) -> Result<Vec<Course>, String> {
    if payload.keyword.is_none() && payload.category.is_none() && payload.creator_address.is_none() && payload.start_date.is_none() && payload.end_date.is_none() {
        return Err("Filter payload is empty; at least one filter criterion must be provided".to_string());
    }
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
                if let Some(start_date) = payload.start_date {
                    matches |= course.created_at >= start_date;
                }
                if let Some(end_date) = payload.end_date {
                    matches |= course.created_at <= end_date;
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
        Err("Couldn't find a course with provided inputs".to_string())
    } else {
        Ok(courses)
    }
}

// Adds a new course to the storage
#[ic_cdk::update]
fn add_course(course: CoursePayLoad) -> Result<Course, String> {
    let address_string: String = api::caller().to_string();
    BANNED_ADDRESSES.with(|banned_addresses| {
        let addresses = banned_addresses.lock().unwrap();
        if addresses.contains(&address_string) {
            Err("User is banned. Cannot add course".to_string())
        } else {
            //Validation Logic
            if course.title.is_empty()
            || course.creator_name.is_empty()
            || course.body.is_empty()
            || course.attachment_url.is_empty()
            || course.keyword.is_empty()
            || course.category.is_empty()
            || course.contact.is_empty()
            {
                return Err("Please fill in all the required fields to create a course".to_string());
            }
            let id = ID_COUNTER
                .with(|counter| {
                    let current_value = *counter.borrow().get();
                    counter.borrow_mut().set(current_value + 1)
                })
                .expect("Cannot increment id counter");
        
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
    })
}

// Updates an existing course. Only the creator or the admin or a moderator can update
#[ic_cdk::update]
fn update_course(id: u64, payload: CourseUpdatePayLoad) -> Result<Course, String> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(mut course) => {
            let caller = api::caller().to_string();
            let is_allowed = _is_allowed(id, caller);
            if is_allowed {
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
            } else {
                Err(format!("You are not authorized to update course with id={}", id))
            }
        }
        None => Err(format!("Couldn't update a course with id={}. Course not found", id)),
    }
}

// Deletes a course based on the ID. Only the creator, admin, or a moderator can delete
#[ic_cdk::update]
fn delete_course(id: u64) -> Result<Course, String> {
    match STORAGE.with(|service| service.borrow().get(&id)) {
        Some(course) => {
            let caller = api::caller().to_string();

            let is_allowed = _is_allowed(id, caller);

            if is_allowed {
                STORAGE.with(|service| service.borrow_mut().remove(&id));
                Ok(course)
            } else {
                Err(format!("You are not authorized to delete course with id={}", id))
            }
        }
        None => Err(format!("Couldn't delete a course with id={}. Course not found", id)),
    }
}

// Deletes all courses by a creator based on the address. Only the admin or a moderator can access
#[ic_cdk::update]
fn delete_courses_by_creator(address: String) -> Result<Vec<Course>, String> {
    let caller = api::caller().to_string();
    let is_allowed = _is_authorized(caller);

    if is_allowed {
        let mut deleted_courses: Vec<Course> = Vec::new();
        STORAGE.with(|service| {
            let mut storage = service.borrow_mut();
            let mut keys_to_remove = Vec::new();
            for (id, course) in storage.iter() {
                if course.creator_address == address {
                    keys_to_remove.push(id.clone());
                    deleted_courses.push(course.clone());
                }
            }
            for key in keys_to_remove {
                storage.remove(&key);
            }
        });
        if deleted_courses.is_empty() {
            Err("No courses found for the caller. Nothing to delete.".to_string())
        } else {
            Ok(deleted_courses)
        }
    } else {
        Err("You are not authorized to delete the courses".to_string())
    }
}

// Deletes all courses of the caller
#[ic_cdk::update]
fn delete_my_courses() -> Result<Vec<Course>, String> {
    let caller = api::caller().to_string();
    let mut deleted_courses: Vec<Course> = Vec::new();

    STORAGE.with(|service| {
        let mut storage = service.borrow_mut();
        let mut keys_to_remove = Vec::new();

        for (id, course) in storage.iter() {
            if course.creator_address == caller {
                keys_to_remove.push(id.clone());
                deleted_courses.push(course.clone());
            }
        }

        for key in keys_to_remove {
            storage.remove(&key);
        }
    });

    if deleted_courses.is_empty() {
        Err("No courses found for the caller. Nothing to delete.".to_string())
    } else {
        Ok(deleted_courses)
    }
}

// Bans a creator from adding courses. Deletes all the courses by the creator. Only the admin or a moderator can access
#[ic_cdk::update]
fn ban_creator(address: String) -> Result<Vec<Course>, String> {
    let caller = api::caller().to_string();

    if _is_authorized(caller) && !_is_authorized(address.clone()) {
        match delete_courses_by_creator(address.clone()){
            Ok(courses) => {
                BANNED_ADDRESSES.with(|banned_addresses| {
                    let mut addresses = banned_addresses.lock().unwrap();
                    addresses.push(address);
                });
                Ok(courses)
            }
            Err(_) => Err("No courses found for the address, cannot ban the user".to_string()),
        }
    } else {
        Err("You are not authorized to ban the user".to_string())
    }
}

// Unban a creator from adding courses. Only the admin or a moderator can access
#[ic_cdk::update]
fn un_ban_creator(address: String) -> Result<(), String> {
    let caller = api::caller().to_string();

    if _is_authorized(caller) {
        BANNED_ADDRESSES.with(|banned_addresses| {
            let mut addresses = banned_addresses.lock().unwrap();
            if let Some(pos) = addresses.iter().position(|x| *x == address) {
                addresses.remove(pos);
                Ok(())
            } else {
                Err("Address not found in banned list.".to_string())
            }
        })
    } else {
        Err("You are not authorized to unban the user".to_string())
    }
}

// Retrieves all courses
#[ic_cdk::query]
fn get_all_courses() -> Vec<Course> {
    STORAGE.with(|storage| {
        storage.borrow().iter().map(|(_, course)| course.clone()).collect()
    })
}

// Counts the number of courses
#[ic_cdk::query]
fn count_courses() -> usize {
    STORAGE.with(|storage| {
        storage.borrow().len().try_into().unwrap()
    })
}

// Internal helper functions

fn _get_course_(id: &u64) -> Option<Course> {
    STORAGE.with(|service| service.borrow().get(id))
}

fn do_insert(course: &Course) {
    STORAGE.with(|service| service.borrow_mut().insert(course.id, course.clone()));
}

fn _is_admin(address: String) -> bool {
    ADMIN_ADDRESS.with(|admin_address| {
        admin_address.lock().unwrap().as_ref() == Some(&address)
    })
}

fn _is_authorized(address: String) -> bool {
    ADMIN_ADDRESS.with(|admin_address| {
        if admin_address.lock().unwrap().as_ref() == Some(&address) {
            true
        } else {
            MODERATOR_ADDRESSES.with(|moderator_addresses| {
                moderator_addresses.lock().unwrap().contains(&address)
            })
        }
    })
}

fn _is_allowed(id: u64, caller: String) -> bool {
    STORAGE.with(|service| {
        if let Some(course) = service.borrow().get(&id) {
            if course.creator_address == caller {
                true
            } else {
                _is_authorized(caller)
            }
        } else {
            false
        }
    })
}

// Error types
#[derive(candid::CandidType, Deserialize, Serialize)]
enum Error {
    NotFound { msg: String },
    UnAuthorized { msg: String },
    EmptyFields { msg: String },
    BannedUser { msg: String }
}

// Need this to generate candid
ic_cdk::export_candid!();
