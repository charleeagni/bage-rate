fn main() {
    let output = std::env::args()
        .nth(1)
        .expect("usage: export_bindings <taurpc.ts>");
    tauri_graphql_transport::export_bindings(&output).expect("export TauRPC TypeScript bindings");
    println!("{output}");
}
