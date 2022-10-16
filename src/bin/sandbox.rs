use chrono::Utc;
use futures::stream::TryStreamExt;
use log::{LevelFilter, info};
use moneta::models::Balance;
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{doc, Document};
use simplelog::{TerminalMode, ColorChoice, SimpleLogger, TermLogger};
use tokio::runtime::Builder;


pub fn initialize_logging(verbosity: i8) {
    let level = match verbosity {
        // 0 => LevelFilter::Warn,
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let mut log_config = simplelog::ConfigBuilder::new();

    // log_config.add_filter_ignore_str("html5ever");
    // log_config.add_filter_ignore_str("hyper");
    // log_config.add_filter_ignore_str("reqwest");
    // log_config.add_filter_ignore_str("selectors");
    // log_config.add_filter_allow_str("moneta");

    if let Err(e) = TermLogger::init(
        level,
        log_config.build(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    ) {
        SimpleLogger::init(level, log_config.build())
            .expect("TermLogger and SimpleLogger failed to init!?");
        info!(
            "Falling back to SimpleLogger because TermLogger init failed: {}",
            e
        );
    }
}


fn main() {
    let runtime = Builder::new_multi_thread()
        .thread_name("moneta-sandbox")
        .enable_all()
        .build()
        .unwrap();

    runtime.block_on(async {
    initialize_logging(1);

        let db = moneta::connect(
            "mongodb://localhost:27017",
            "moneta_sandbox",
            "moneta-sandbox",
        )
        .await
        .unwrap();

        // List the names of the collections in that database.
        for collection_name in db.list_collection_names(None).await.unwrap() {
            println!("{}", collection_name);
        }

        // Get a handle to a collection in the database.
        let collection = db.collection::<Document>("books");

        let docs = vec![
            doc! { "title": "1984", "author": "George Orwell" },
            doc! { "title": "Animal Farm", "author": "George Orwell" },
            doc! { "title": "The Great Gatsby", "author": "F. Scott Fitzgerald" },
        ];

        // Insert some documents into the "my_db.books" collection.
        let res = collection.insert_many(docs, None).await.unwrap();
        println!("{:?}", res);

        // Get a handle to a collection of `Book`.
        let typed_collection = db.collection::<Balance>("balances");

        let balances = vec![
            Balance {
                account_id: ObjectId::new().into(),
                id: ObjectId::new().into(),
                date: Utc::now(),
                amount: 1234,
                comments: "".to_string(),
            },
            Balance {
                account_id: ObjectId::new().into(),
                id: ObjectId::new().into(),
                date: Utc::now(),
                amount: 1234,
                comments: "".to_string(),
            },
        ];
        println!("to insert: {:?}", balances);
        println!("to insert: {:?}", serde_json::to_string(&balances[0]));

        // Insert the books into "my_db.books" collection, no manual conversion to BSON necessary.
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
