
use futures_util::{TryStreamExt};
use mongodb::{bson::{Document, Bson}, Client};
use std::{fs::File, io::Read, io, collections::HashMap};
use flate2::read::GzDecoder;
use tar::Archive;
use similar::{TextDiff};

use crate::connection::*;
use crate::globals::*;
use utils::utils;
use chrono;

pub struct Processor {
  file_path: String,
  conn: Connection,
  db_client: Client,
  config: HashMap<String, String>
}


impl Processor {

  pub fn new(file_path: String, conn: Connection, db_client: Client) -> Processor {
    Processor { file_path, conn, db_client, config: Globals::new().globals}
  }

  

  fn decompress_data(&self, doc: &mut Document) -> io::Result<()> {
    
    let mut unzipper = GzDecoder::new(File::open(&self.file_path)?);
    let mut uncompressed: Vec<u8> = Vec::new();
    unzipper.read_to_end(&mut uncompressed)?;

    let mut archive = Archive::new(uncompressed.as_slice());

    let file_exts: Vec<&str> = self.config.get("tracked_files").unwrap().split_whitespace().collect();

    for file in archive.entries()? {
      let mut file = file?;

      let filename = file.path()?.into_owned().to_str().unwrap().to_string();
      
      // only insert file if extension is in "tracked_files"
      for s in &file_exts {
        if filename.contains(s) || filename.contains(".rev") || filename.contains("watch") {
          let mut buf = String::new();
          file.read_to_string(&mut buf)?;
          doc.insert(filename, buf);
          break;
        }
      }
    }

    Ok(())
  }

  async fn get_file_diffs(&self, diffs:&mut Document, file_doc: &Document, rev_file_hash: HashMap<String, String>, db_name: &str, coll_name: &str) -> std::result::Result<(), mongodb::error::Error> {
    // first getting entry whose id matches the new parent id
    let parent_id = rev_file_hash.get("parent_id").unwrap();  
    if parent_id != "*" {

      // get parent rev hash
      let mut res = Connection::simple_db_query(&self.db_client, "id", parent_id, db_name, coll_name).await;

      // checks for case where parent id no longer exists
      let parent = match res.try_next().await? {
        Some(v) => v,
        None => {
          println!("No matching parent id in database, may have incorrectly deleted a record");
          return Ok(())
        }
      };

      let parent_rev = parent.get("files").unwrap().as_document().unwrap().get(".rev").unwrap().as_str().unwrap();
      let mut parent_hash = HashMap::new();
      utils::read_file_into_hash(&parent_rev, None, &mut parent_hash).unwrap();

      // finding which files changed
      let mut modified_files: Vec<&str> = Vec::new();
      for (k,v) in &rev_file_hash {
        
        // don't care about comparing id and parent id
        if k == "id" || k == "parent_id" {
          continue;
        }

        let old_hash = match parent_hash.get(k) {
          Some(record) => record,
          None => "None"
        };

        if old_hash != v {
          modified_files.push(k);
        }

      }

      // Diffing the modified files
      for file in modified_files {
        
        let new_file = file_doc.get(file).unwrap().as_str().unwrap();
        
        // this gets the file from the database
        let old_file = match parent.get("files").unwrap().as_document().unwrap().get(file) {
          Some(f) => f.as_str().unwrap(),
          None => ""
        };

        let full_diff = TextDiff::from_lines(old_file, new_file);
        let mut diffs_file_chg = Document::new();
        for (diff_idx, chg) in full_diff.unified_diff().header("old_file", "new_file").iter_hunks().enumerate() {
          diffs_file_chg.insert(diff_idx.to_string(), chg.to_string());
        }
        diffs.insert(file, diffs_file_chg);
        
      }

    }

    Ok(())
  }

  pub async fn process_data(&self) -> std::result::Result<(), mongodb::error::Error> {
    
    let mut parent_doc = Document::new();
    let db_name = self.config.get("database").unwrap();
    let coll_name = self.config.get("registry").unwrap();

    // inserting general upload metadata
    parent_doc.insert("upload_hash", &self.conn.filehash);
    parent_doc.insert("upload_name", &self.conn.filename);
    parent_doc.insert("upload_path", &self.file_path);
    parent_doc.insert("upload_time", chrono::offset::Utc::now());
    
    // Decompressing file and getting tracked and .rev files
    let mut file_doc = Document::new();
    self.decompress_data(&mut file_doc).expect("Decompression failed");
    let mut rev_file_hash = HashMap::new();
    utils::read_file_into_hash(file_doc.get(".rev").unwrap().as_str().unwrap(), None, &mut rev_file_hash)?;
    parent_doc.insert("id", rev_file_hash.get("id").unwrap());
    parent_doc.insert("parent_id", rev_file_hash.get("parent_id").unwrap());

    // Calculating diffed files
    let mut diffs = Document::new();
    self.get_file_diffs(&mut diffs, &file_doc, rev_file_hash, db_name, coll_name).await?;

    

    // Getting all desired values from files
    let mut watch_schema: HashMap<String, String> = HashMap::new();
    utils::read_file_into_hash(file_doc.get("watch").unwrap().as_str().unwrap(), None, &mut watch_schema)?;

    let mut watch_values = Document::new();

    for (file,contents) in &file_doc {

      if file == "watch" { continue; } // skipping watch file

      for line in contents.as_str().unwrap().lines() {
        let l: Vec<&str> = line.split_whitespace().collect();

        for (watch_name, watch_type) in &watch_schema {
          if l.contains(&watch_name.as_str()) {

            let val_pos = l.iter().position(|&x| x == watch_name).unwrap() + 1;

            match watch_type.as_str() {
              "int" => {
                // this will skip occurrences of the watch variable with the wrong type. Kind of a hack for now
                let val = l[val_pos].parse::<i64>();
                match val {
                  Ok(v) => { watch_values.insert(watch_name.to_string(), v); },
                  Err(_e) => println!("Incorrect type, skipping")
                };
                
              },
              "float" => {
                let val: f64  = l[val_pos].parse().unwrap();
                watch_values.insert(watch_name.to_string(), val);
              },
              "long_string" => { // string with spaces
                watch_values.insert(watch_name.to_string(), l[val_pos..].join(" "));
              },
              "string" => {
                watch_values.insert(watch_name.to_string(), l[val_pos]);
              },
              &_ => { // anything is simple string
                println!("Error! Invalid watch type {}", watch_type);
              }

            };
            

          }
        }
      }
    }

    // get thermo log data
    match watch_schema.get("thermo") {
      Some(_) => {

        let f = file_doc.get("log.lammps").unwrap().as_str().unwrap(); // assuming for now log file is just "log.lammps"
        let mut thermo_keys: Vec<&str> = Vec::new(); // this vec is used to keep the correct order of data
        let mut thermo_data = Document::new(); //HashMap<&str, Vec<f64>> = HashMap::new();
        let mut read_mode = false;
        let mut read_count = 0;

        for line in f.lines() {

          let l: Vec<&str> = line.split_whitespace().collect();

          // need to know when to stop. So will break when we no longer find numbers
          // can clear out vec and doc to use for other runs
          if read_mode {
            match l[0].parse::<f64>() {
              Ok(_) => (),
              Err(_) => {
                let mut doc_key = "thermo_data_".to_string();
                doc_key.push_str(&read_count.to_string());
                watch_values.insert(doc_key.as_str(), thermo_data.clone());
                thermo_keys.clear();
                thermo_data.clear();
                read_mode = false;
                read_count += 1;
              }
            };
          }

          // thermo data begins with "step". should get names of thermo data from this line
          if l.contains(&"Step") {
            for s in l {
              thermo_keys.push(s);
              let  v: Vec<f64> = Vec::new();
              thermo_data.insert(s, v);
            }
            read_mode = true;
            continue;
          }

          if read_mode {
            for (i,s) in l.iter().enumerate() {
              thermo_data.get_array_mut(thermo_keys[i]).unwrap().push(Bson::Double(s.parse().unwrap())); // yea bson is wild
            }
          }
        }

        

      },
      None => ()
    };

    parent_doc.insert("watch", watch_values);
    parent_doc.insert("files", file_doc);
    parent_doc.insert("diffs", diffs);
    
    
    let db = self.db_client.database(db_name);
    let collection = db.collection::<Document>(coll_name);
    collection.insert_one(parent_doc, None).await?;

    Ok(())
    
  }

}