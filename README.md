# Fuel Cost Server

A Rust REST API server for tracking fuel costs with email/password authentication. Data is stored in SQLite with JSON serialization for flexible fuel entry storage.

## Features

- **Email/Password Authentication**: Simple signup and signin
- **Auto-account Creation**: Creates account automatically if user doesn't exist during signin
- **Fuel Entry Management**: Full CRUD operations for fuel entries
- **JSON Storage**: Fuel entries stored as JSON strings in database for flexibility
- **SQLite Database**: Lightweight, file-based database
- **CORS Support**: Cross-origin resource sharing enabled

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable version)
- [Git](https://git-scm.com/)

## Installation & Setup

1. **Clone the repository**
   ```bash
   git clone <repository-url>
   cd fuel_cost_server
   ```

2. **Install dependencies and run**
   ```bash
   # Option 1: Use the provided script (recommended)
   ./run_server.sh
   
   # Option 2: Manual build and run
   cargo build
   cargo run
   ```

The server will start on `http://localhost:3001`

**Note**: The SQLite database file (`fuel_cost.db`) will be created automatically on first run.

## API Endpoints

All API responses follow a consistent format. Successful responses return the requested data, while error responses return structured error information:

```json
{
  "error": "Brief error description",
  "details": "Detailed error message with specific information"
}
```

### Authentication

#### Sign Up
- **POST** `/api/auth/signup`
- **Body**: 
  ```json
  {
    "email": "user@example.com",
    "password": "password123"
  }
  ```
- **Response**: 
  ```json
  {
    "user_id": "uuid-string",
    "email": "user@example.com"
  }
  ```

#### Sign In
- **POST** `/api/auth/signin`
- **Body**: 
  ```json
  {
    "email": "user@example.com",
    "password": "password123"
  }
  ```
- **Response**: 
  ```json
  {
    "user_id": "uuid-string",
    "email": "user@example.com"
  }
  ```
- **Note**: If user doesn't exist, account will be created automatically

### Fuel Entries

#### Create Fuel Entry (Single)
- **POST** `/api/fuel-entries`
- **Body**: 
  ```json
  {
    "user_id": "uuid-string",
    "liters": 50.5,
    "price_per_liter": 1.45,
    "total_cost": 73.23,
    "date_time": "2024-01-15T10:30:00Z",
    "odometer_reading": 125000.5
  }
  ```

#### Create Fuel Entries (Bulk)
- **POST** `/api/fuel-entries/bulk`
- **Body**: 
  ```json
  {
    "user_id": "uuid-string",
    "entries": [
      {
        "liters": 45.5,
        "price_per_liter": 1.42,
        "total_cost": 64.61,
        "date_time": "2024-01-10T08:30:00Z",
        "odometer_reading": 124500.0
      },
      {
        "liters": 52.3,
        "price_per_liter": 1.45,
        "total_cost": 75.84,
        "date_time": "2024-01-15T14:20:00Z",
        "odometer_reading": 124950.5
      }
    ]
  }
  ```
- **Response**: 
  ```json
  {
    "message": "Successfully created 2 fuel entries",
    "count": 2,
    "entries": [/* array of created entries */]
  }
  ```

#### Get All Fuel Entries
- **GET** `/api/fuel-entries/{user_id}`
- **Response**: Array of fuel entry objects

#### Get Specific Fuel Entry
- **GET** `/api/fuel-entries/{user_id}/{fuel_entry_id}`
- **Response**: Single fuel entry object

#### Update Fuel Entry
- **PUT** `/api/fuel-entries/{user_id}/{fuel_entry_id}`
- **Body**: (partial updates supported)
  ```json
  {
    "liters": 52.0,
    "total_cost": 75.40
  }
  ```

#### Delete Fuel Entry
- **DELETE** `/api/fuel-entries/{user_id}/{fuel_entry_id}`
- **Response**: 
  ```json
  {
    "message": "Fuel entry deleted successfully"
  }
  ```

#### Delete Fuel Entries (Bulk)
- **POST** `/api/fuel-entries/bulk/delete`
- **Request Body**:
  ```json
  {
    "user_id": "string",
    "entry_ids": ["string", "string", ...]
  }
  ```
- **Response**:
  ```json
  {
    "message": "Successfully deleted 2 fuel entries",
    "deleted_count": 2,
    "total_requested": 3,
    "not_found_count": 1,
    "deleted_ids": ["entry-uuid-1", "entry-uuid-2"]
  }
  ```
- **Note**: Transaction-safe operation. Only deletes entries that exist and belong to the user. Returns detailed results including which entries were successfully deleted.

## Data Models

### User
```json
{
  "id": "string",
  "email": "string",
  "password_hash": "string",
  "created_at": "datetime"
}
```

### Fuel Entry
```json
{
  "id": "string",
  "user_id": "string",
  "liters": "number",
  "price_per_liter": "number",
  "total_cost": "number",
  "date_time": "datetime",
  "odometer_reading": "number (optional)"
}
```

## Database Schema

The database uses a simplified schema with JSON storage:

### users table
- `id` (TEXT PRIMARY KEY)
- `email` (TEXT UNIQUE NOT NULL)
- `password_hash` (TEXT NOT NULL)
- `created_at` (TEXT NOT NULL)

### fuel_entries table
- `id` (TEXT PRIMARY KEY)
- `user_id` (TEXT NOT NULL) - Foreign key to users table for efficient filtering
- `data` (TEXT NOT NULL) - JSON string containing all fuel entry data

## Testing with Postman

1. Import the `Fuel_Cost_API.postman_collection.json` file into Postman
2. Set the `base_url` variable to `http://localhost:3001`
3. Use the collection to test all endpoints

### Quick Test Flow:
1. **Sign up** a new user → copy `user_id` from response
2. **Create fuel entry** or **Create bulk entries** → copy `id`(s) from response
3. **Get all entries** for the user
4. **Update** a fuel entry
5. **Delete** a single entry or **Delete bulk entries**

## Development

### Project Structure
```
src/
├── main.rs          # Server setup and routing
├── models.rs        # Data structures and request/response models
├── database.rs      # Database operations and queries
├── auth.rs          # Password hashing and verification
└── handlers.rs      # HTTP request handlers
```

### Building
```bash
cargo build
```

### Running in Development
```bash
cargo run
```

### Running Tests
```bash
# Run the test script (if available)
./test_api.sh
```

## Configuration

- **Database**: SQLite file (`fuel_cost.db`) created automatically
- **Server Port**: 3001 (configurable in `main.rs`)
- **CORS**: Permissive (allows all origins)

## Troubleshooting

### Database Issues
- **Error**: "unable to open database file"
  - **Solution**: The server will now create the database automatically. Ensure you have write permissions in the project directory.

### Build Issues
- **Error**: "cargo not found"
  - **Solution**: Install Rust using `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

### Server Issues
- **Error**: "Address already in use"
  - **Solution**: Stop any existing server with `pkill -f fuel_cost_server` or change the port in `main.rs`

### Testing Issues
- **Error**: Connection refused
  - **Solution**: Ensure the server is running on `http://localhost:3001`

### API Error Messages
Common error responses and their meanings:

#### Authentication Errors
- **409 Conflict**: "User already exists" - Email is already registered
- **401 Unauthorized**: "Invalid credentials" - Wrong password
- **500 Internal Server Error**: Database or password hashing issues

#### Fuel Entry Errors  
- **400 Bad Request**: "Invalid user ID" - User doesn't exist in database
- **400 Bad Request**: "Empty entries list" - Bulk creation with no entries
- **400 Bad Request**: "Empty entry IDs list" - Bulk deletion with no entry IDs
- **404 Not Found**: "Fuel entry not found" - Entry doesn't exist or wrong user
- **422 Unprocessable Entity**: Missing required fields (user_id, liters, etc.)
- **500 Internal Server Error**: Database operation failed

**Note**: Bulk operations use database transactions - if any entry fails, all entries in the batch are rolled back.

All error responses include detailed `details` field with specific information to help debug the issue.

## Files Structure
```
fuel_cost_server/
├── src/
│   ├── main.rs              # Server setup and routing
│   ├── models.rs            # Data structures
│   ├── database.rs          # Database operations
│   ├── auth.rs              # Password handling
│   └── handlers.rs          # HTTP handlers
├── fuel_cost.db             # SQLite database (auto-created)
├── run_server.sh            # Startup script
├── test_api.sh              # API testing script
├── README.md                # This file
└── *.postman_collection.json # Postman collections
```

## Security Notes

- Passwords are hashed using bcrypt
- No JWT tokens - simplified authentication
- User access controlled by user_id parameter in requests
- Database queries use parameterized statements to prevent SQL injection
- Database file excluded from git via .gitignore

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]