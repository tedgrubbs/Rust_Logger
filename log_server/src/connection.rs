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

  pub async fn simple_db_query(client: &Client, field: Option<&str>, value: Option<&str>, db: &str, collection: &str, return_fields: Option<Document>, sort_field: Option<Document>) -> mongodb::Cursor<Document> {

    // default to a "select *" query
    let filter = match field.is_some() {
      true => Some(doc! {field.unwrap(): value.unwrap() }),
      false => None
    };
    
   
    let db = client.database(db).collection::<Document>(collection);    

    let find_options = FindOptions::builder();

    // defaults to returning just the id field
    let find_options = match return_fields {
      Some(doc) => find_options.projection(doc),
      None => find_options.projection(doc! { "id": 1 })
    };


    let find_options = match sort_field {
      Some(doc) => find_options.sort(doc),
      None => find_options.sort(doc! {})
    };

    return db.find(filter, find_options.build()).await.unwrap()
  }

}