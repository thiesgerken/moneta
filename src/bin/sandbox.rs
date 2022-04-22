use chrono::{DateTime, Utc};
use moneta::models::Balance;
// use moneta::schema::*;
use moneta::initialize_logging;
use tokio::runtime::Builder;

use futures::stream::TryStreamExt;
use mongodb::{bson::oid::ObjectId, options::ClientOptions, Client};
use uuid::Uuid;
// use log::info;

fn main() {
    let runtime = Builder::new_multi_thread()
        .thread_name("moneta-sandbox")
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
        initialize_logging(1);

        // Parse a connection string into an options struct.
        let mut client_options = ClientOptions::parse("mongodb://localhost:27017")
            .await
            .unwrap();

        // Manually set an option.
        client_options.app_name = Some("My App".to_string());

        // Get a handle to the deployment.
        let client = Client::with_options(client_options).unwrap();

        // List the names of the databases in that deployment.
        for db_name in client.list_database_names(None, None).await.unwrap() {
            println!("{}", db_name);
        }

        // Get a handle to a database.
        let db = client.database("mydb");

        // List the names of the collections in that database.
        for collection_name in db.list_collection_names(None).await.unwrap() {
            println!("{}", collection_name);
        }

        use mongodb::bson::{doc, Document};

        // Get a handle to a collection in the database.
        let collection = db.collection::<Document>("books");

        let docs = vec![
            doc! { "title": "1984", "author": "George Orwell" },
            doc! { "title": "Animal Farm", "author": "George Orwell" },
            doc! { "title": "The Great Gatsby", "author": "F. Scott Fitzgerald" },
        ];

        // Insert some documents into the "mydb.books" collection.
        let res = collection.insert_many(docs, None).await.unwrap();
        println!("{:?}", res);

        // Get a handle to a collection of `Book`.
        let typed_collection = db.collection::<Balance>("balances");

        let balances = vec![Balance {
            account_id: 24,
            id: ObjectId::new(),
            date: Utc::now(),
            amount: 1234,
            comments: "".to_string(),
        }];
        println!("to insert: {:?}", balances);

        // Insert the books into "mydb.books" collection, no manual conversion to BSON necessary.
        let res = typed_collection.insert_many(balances, None).await.unwrap();
        println!("insert result: {:?}", res);

        // Query the books in the collection with a filter and an option.
        let filter = doc! {};
        let mut cursor = typed_collection.find(filter, None).await.unwrap();

        // Iterate over the results of the cursor.
        while let Some(balance) = cursor.try_next().await.unwrap() {
            println!("balance: {:?}", balance);
        }
    });
}
