//! Builds the window's context and the permissions of its commands: the interface may call only
//! the commands listed here, and the capability in `capabilities/` grants exactly these.

/// The commands of the window (see `src/commands.rs`).
const COMMANDS: &[&str] = &[
    "startup",
    "overview",
    "sources",
    "taxonomy",
    "search",
    "newest",
    "tally",
    "tools",
    "private_status",
    "private_create",
    "private_unlock",
    "private_lock",
    "private_recover",
    "private_change_password",
    "document",
    "import_addresses",
    "proposals",
    "decision_options",
    "confirm_proposal",
    "reject_proposal",
    "pending_share",
    "handed_over",
    "receive_list",
    "sync_shared_register",
    "forget",
    "crawl_start",
    "crawl_stop",
    "crawl_status",
    "get_settings",
    "set_settings",
    "open_link",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("the window's context and permissions");
}
