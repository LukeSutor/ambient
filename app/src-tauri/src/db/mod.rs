pub mod core;      // connection, migrations, generic SQL helpers
pub mod events;    // event storage & retrieval
pub mod workflows; // workflow CRUD
pub mod activity;  // activity summaries

// Intentionally no re-exports to enforce explicit module paths after refactor.
