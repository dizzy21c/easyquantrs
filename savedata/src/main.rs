// use mongodb::{Client, options::ClientOptions};
// use mongodb::bson::{doc, Document};
// use serde::{Deserialize, Serialize};
use mongodb::{
    bson::doc,
    bson::Document,
    sync::Client,
    options::InsertManyOptions,
};
use serde::{Deserialize, Serialize};
// use mongodb::bson::{doc, Document};

struct Err {}
impl From<mongodb::error::Error> for Err {
    fn from(_error: mongodb::error::Error) -> Self {
        Err {}
    }
}

#[allow(dead_code)]
type Result<T> = std::result::Result<T, Err>;


// #[tokio::main]
fn main() {
    println!("Hello, world!");
    let client = Client::with_uri_str("mongodb://localhost:27017").unwrap();
    // for db_name in client.list_database_names(None, None).unwrap() {
    //     println!("{}", db_name);
    // }

    let database = client.database("quantaxis");
    // for collection_name in database.list_collection_names(None).unwrap() {
    //     println!("{}", collection_name);
    // }

    let collection = database.collection::<Document>("books");
    // let option = InsertManyOptions {
    //     bypass_document_validation: true,
    //     ordered:false,
    //     // write_concern:
    // };
    let docs = vec![
        doc! { "_id":"123456", "title": "1984", "author": "George Orwell" },
        doc! { "title": "Animal Farm", "author": "George Orwell" },
        doc! { "title": "The Great Gatsby", "author": "F. Scott Fitzgerald" },
    ];
    collection.delete_many(doc!{"_id":"123456"}, None);
    collection.insert_many(docs, None);//option);

    // let collection = database.collection::<Book>("books");

}
