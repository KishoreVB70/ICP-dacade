# icp_rust_learning_platform
This project is a decentralized learning platform built on the Internet Computer, inspired from medium and dacade. It allows users to create, read, delete, and update courses, with certain roles and permissions to ensure security and proper management.

This project leverages the capabilities of the Internet Computer to provide a decentralized, permission-based system for managing online courses, ensuring robust access control and user management.

### Key Features

1. **Course Management**
   - **Add Course:** Users can add new courses with details like title, creator name, body, attachment URL, keyword, category, and contact information.
   - **Update Course:** Only the creator, admin, or moderators can update a course's details.
   - **Delete Course:** Courses can be deleted by the creator, admin, or moderators.
   - **Delete My Courses:** Users can delete all their own courses.
   - **Delete Courses by Creator:** Admins and moderators can delete all courses by a specific creator.

2. **Course Filtering**
    - AND based filtering provides the courses which match all of the criterias of the user
    - OR based filtering provided courses whcih match any of the criterias fo the user
   - **Filter Courses (AND Condition):** Retrieve courses that match all provided criteria (keyword, category, creator address).
   - **Filter Courses (OR Condition):** Retrieve courses that match any of the provided criteria (keyword, category, creator address).

3. **User Roles and Permissions**
   - To regulate ill actors, a moderation system is created based on admin access
   - **Admin Management:** 
     - Set or change the admin address.
     - Admin has the highest level of permissions, mainly changing the admin and adding, removing moderators.
   - **Moderator Management:** 
     - Add and remove moderators.
     - Moderators can manage courses(update, delete) and users(ban, unban) but have limited permissions compared to the admin.
   - **Banned Users Management:** 
     - Ban users from adding courses.
     - Unban users.
     - Banning a user also removes all their courses.


4. **Error Handling**
   - **Not Found:** Returns an error if a course or user is not found.
   - **Unauthorized Access:** Returns an error if a user tries to perform an action without the necessary permissions.
   - **Empty Fields:** Returns an error if required fields are missing during course creation.
   - **Banned User:** Returns an error if a banned user tries to add a course.

### Detailed Functionality

1. **Set Admin Address**
   - Initializes the admin address if not already set or allows the current admin to change it.

2. **Add Moderator**
   - Allows the admin to add a new moderator, with a maximum of 5 moderators.

3. **Remove Moderator**
   - Allows the admin to remove a moderator.

4. **Get Course**
   - Retrieves a course based on its ID.

5. **Add Course**
   - Allows users to add a new course if they are not banned and have provided all required fields.

6. **Update Course**
   - Allows the course creator, admin, or moderators to update course details.

7. **Delete Course**
   - Allows the course creator, admin, or moderators to delete a course.

8. **Delete Courses by Creator**
   - Allows the admin or moderators to delete all courses by a specific creator.

9. **Delete My Courses**
   - Allows users to delete all courses they have created.

10. **Ban Creator**
    - Allows the admin or moderators to ban a user from adding courses and deletes all their courses.

11. **Unban Creator**
    - Allows the admin or moderators to unban a user.

12. **Filter Courses (AND Condition)**
    - Retrieves courses that satisfy all provided filter criteria.

13. **Filter Courses (OR Condition)**
    - Retrieves courses that satisfy any of the provided filter criteria.

### Internal Helper Functions

- **_get_course_:** Internal function to retrieve a course from storage by ID.
- **do_insert:** Internal function to insert or update a course in storage.
- **_is_admin:** Checks if a given address is the admin.
- **_is_authorized:** Checks if a given address is either the admin or a moderator.
- **_is_allowed:** Checks if the caller is the creator of the course, admin, or a moderator.

### Error Types

- **NotFound:** Error type for when a course or user is not found.
- **UnAuthorized:** Error type for unauthorized access attempts.
- **EmptyFields:** Error type for missing required fields during course creation.
- **BannedUser:** Error type for actions attempted by banned users.

## Requirements
* rustc 1.64 or higher
```bash
$ curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
$ source "$HOME/.cargo/env"
```
* rust wasm32-unknown-unknown target
```bash
$ rustup target add wasm32-unknown-unknown
```
* candid-extractor
```bash
$ cargo install candid-extractor
```
* install `dfx`
```bash
$ DFX_VERSION=0.15.0 sh -ci "$(curl -fsSL https://sdk.dfinity.org/install.sh)"
$ echo 'export PATH="$PATH:$HOME/bin"' >> "$HOME/.bashrc"
$ source ~/.bashrc
$ dfx start --background
```

If you want to start working on your project right away, you might want to try the following commands:

```bash
$ cd icp_rust_boilerplate/
$ dfx help
$ dfx canister --help
```

## Update dependencies

update the `dependencies` block in `/src/{canister_name}/Cargo.toml`:
```
[dependencies]
candid = "0.9.9"
ic-cdk = "0.11.1"
serde = { version = "1", features = ["derive"] }
serde_json = "1.0"
ic-stable-structures = { git = "https://github.com/lwshang/stable-structures.git", branch = "lwshang/update_cdk"}
```

## did autogenerate

Add this script to the root directory of the project:
```
https://github.com/buildwithjuno/juno/blob/main/scripts/did.sh
```

Update line 16 with the name of your canister:
```
https://github.com/buildwithjuno/juno/blob/main/scripts/did.sh#L16
```

After this run this script to generate Candid.
Important note!

You should run this script each time you modify/add/remove exported functions of the canister.
Otherwise, you'll have to modify the candid file manually.

Also, you can add package json with this content:
```
{
    "scripts": {
        "generate": "./did.sh && dfx generate",
        "gen-deploy": "./did.sh && dfx generate && dfx deploy -y"
      }
}
```

and use commands `npm run generate` to generate candid or `npm run gen-deploy` to generate candid and to deploy a canister.

## Running the project locally

If you want to test your project locally, you can use the following commands:

```bash
# Starts the replica, running in the background
$ dfx start --background

# Deploys your canisters to the replica and generates your candid interface
$ dfx deploy
```