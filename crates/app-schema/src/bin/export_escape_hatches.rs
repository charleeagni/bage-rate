//! Every escape-hatch operation each Module declared, as JSON.
//!
//! The declaration lives in Rust — a Module names one through a registration
//! call rather than a comment — so the guard that counts them and demands a
//! written exception for each reads them from here rather than by grepping
//! for a call shape that a rename would silently defeat.

fn main() {
    let modules = module_registry::modules();
    let entries: Vec<String> = modules
        .iter()
        .map(|module| {
            let hatches: Vec<String> = module
                .escape_hatches()
                .iter()
                .map(|hatch| format!("\"{hatch}\""))
                .collect();
            format!("  \"{}\": [{}]", module.name(), hatches.join(", "))
        })
        .collect();
    println!("{{\n{}\n}}", entries.join(",\n"));
}
