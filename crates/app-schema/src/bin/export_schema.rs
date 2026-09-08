use app_schema::{
    database::in_memory_database,
    schema::{app_schema, module_schema},
};

#[tokio::main]
async fn main() {
    let mut module: Option<String> = None;
    let mut output: Option<String> = None;
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--module" {
            module = Some(arguments.next().expect("--module requires a module name"));
        } else {
            output = Some(argument);
        }
    }
    let output = output.expect("usage: export_schema [--module <name>] <output.graphql>");
    let database = in_memory_database()
        .await
        .expect("migrate the schema database");
    let schema = match module {
        None => app_schema(database).expect("build the composed Seaography schema"),
        Some(name) => module_schema(database, &name).expect("build the module mini-schema"),
    };

    std::fs::write(&output, schema.sdl()).expect("write the generated SDL");
    println!("{output}");
}
