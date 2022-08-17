use hyper::{Request, Body};
use mongodb::{bson::{Document}, Client, bson::doc, options::FindOptions};


#[derive(Debug)]
pub struct Connection {
  pub username: String,
  pub password: String,
  pub filename: String,
  pub collection: String,
  pub filehash: String
}

impl Connection {
  pub fn get_conn_info(req: &Request<Body>) -> std::result::Result<Connection, String> {

    let headers = req.headers();

    let username = match headers.get("username") {
      Some(k) => String::from(k.to_str().unwrap()),
      None => return Err("No username provided".to_string())
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

    Ok (
      Connection {
        username,
        password,
        filename,
        collection,
        filehash
      }
    )
    
  }

  pub async fn simple_db_query(client: &Client, field: &str, value: &str, db: &str, collection: &str, return_all_fields: Option<bool>) -> mongodb::Cursor<Document> {

    let filter = doc! {field: value };
    let db = client.database(db).collection::<Document>(collection);    

    let find_options = match return_all_fields {
      Some(b) => {
        if b {
          FindOptions::builder().projection(doc! { "files": 1 }).build()
        } else {
          FindOptions::builder().projection(doc! { "id": 1 }).build()

        }
      },
      None => FindOptions::builder().projection(doc! { "id": 1 }).build()

    };

    return db.find(filter, find_options).await.unwrap()
  }

}