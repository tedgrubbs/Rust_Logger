use hyper::{Request, Body};
use mongodb::{bson::{Document}, Client, bson::doc, options::FindOptions};


// The Connection struct is used to manage information from any incoming requests.


#[derive(Debug,Clone)]
pub struct Connection {
  pub username: String,
  pub password: String,
  pub filename: String,
  pub collection: String,
  pub filehash: String,
  pub err: Option<String>
}

impl Connection {
  pub fn get_conn_info(req: &Request<Body>) -> Connection {

    let headers = req.headers();

    let username = match headers.get("username") {
      Some(k) => String::from(k.to_str().unwrap()),
      None => String::from("admin")
    };

    let password = match headers.get("password") {
      Some(k) => String::from(k.to_str().unwrap()),
      None => String::new()
    };

    let filename = match headers.get("filename") {
      Some(k) => String::from(k.to_str().unwrap()),
      None => String::new()
    };

    let filehash = match headers.get("filehash") {
      Some(k) => String::from(k.to_str().unwrap()),
      None => String::new()
    };

    let collection = match headers.get("collection") {
      Some(k) => String::from(k.to_str().unwrap()),
      None => String::new()
    };

    
    Connection {
      username,
      password,
      filename,
      collection,
      filehash,
      err: None
    }
    
    
  }

  pub async fn simple_db_query(client: &Client, field: &str, value: &str, db: &str, collection: &str, return_all_fields: Option<Document>) -> mongodb::Cursor<Document> {

    let filter = doc! {field: value };
    let db = client.database(db).collection::<Document>(collection);    

    let find_options = match return_all_fields {
      Some(doc) => FindOptions::builder().projection(doc).build(),
      None => FindOptions::builder().projection(doc! { "id": 1 }).build()
    };

    return db.find(filter, find_options).await.unwrap()
  }

}