use hyper::{Request, Body};
use mongodb::{bson::{Document}, Client, bson::doc};


#[derive(Debug)]
pub struct Connection {
  pub username: String,
  pub password: String,
  pub filename: String,
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

    Ok (
      Connection {
        username,
        password,
        filename,
        filehash
      }
    )
    
  }

  pub async fn simple_db_query(client: &Client, field: &str, value: &str, db: &str, collection: &str) -> mongodb::Cursor<Document> {

    let filter = doc! {field: value };
    let db = client.database(db).collection::<Document>(collection);
    let cursor = db.find(filter, None).await.unwrap();
    return cursor
  }

}