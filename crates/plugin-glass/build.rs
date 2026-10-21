// Swift commands are invoked straight from JS (`plugin:glass|<name>`); each
// one still needs a permission, and the listener pair is what
// `addPluginListener` calls.
const COMMANDS: &[&str] = &[
    "is_available",
    "show",
    "update",
    "hide",
    "buttons",
    "actions",
    "register_listener",
    "remove_listener",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).ios_path("ios").build();
}
