use moneta::{connect, initialize_logging};

fn main() {
    initialize_logging(2);

    let connection =
        connect("postgres://localhost/moneta").expect("error connecting and migrating to db");

    println!("{:?}", moneta::queries::relevant_accounts(&connection, 1));
}
