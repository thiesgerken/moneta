use uuid::Uuid;
use chrono::Utc;
use futures::stream::TryStreamExt;
use log::{LevelFilter, info};
use moneta::models::Balance;
use mongodb::bson::doc;
use simplelog::{ColorChoice, SimpleLogger, TermLogger, TerminalMode};
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

        let db = moneta::connect_mongo(
            "mongodb://localhost:27017",
            "moneta_sandbox",
            "moneta-sandbox",
        )
        .await
        .unwrap();

        // List the names of the collections in that database.
        for collection_name in db.list_collection_names().await.unwrap() {
            println!("{}", collection_name);
        }

        let typed_collection = db.collection::<Balance>("balances");

        let balances = vec![
            Balance {
                account_id: Uuid::now_v7().into(),
                id: Uuid::now_v7().into(),
                date: Utc::now(),
                amount: 1234,
                comments: "".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Balance {
                account_id: Uuid::now_v7().into(),
                id: Uuid::now_v7().into(),
                date: Utc::now(),
                amount: 1234,
                comments: "".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        println!("to insert: {:?}", balances);
        println!("to insert: {:?}", serde_json::to_string(&balances[0]));

        let res = typed_collection.insert_many(balances).await.unwrap();
        println!("insert result: {:?}", res);

        let filter = doc! {};
        let mut cursor = typed_collection.find(filter).await.unwrap();

        // Iterate over the results of the cursor.
        while let Some(balance) = cursor.try_next().await.unwrap() {
            println!("balance: {:?}", balance);
        }
    });
}
