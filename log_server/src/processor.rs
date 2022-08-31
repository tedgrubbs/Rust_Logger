
use futures_util::{TryStreamExt};
use mongodb::{bson::{Document, Bson, Array, doc}, Client};
use std::{fs::File, io::Read, io, collections::HashMap};
use flate2::read::GzDecoder;
use tar::Archive;
use similar::{TextDiff};

use crate::connection::*;
use crate::config::*;
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
    Processor { file_path, conn, db_client, config: Config::new().config}
  }

  

  fn decompress_data(&self, doc: &mut Document, watch_values: &mut Document) -> io::Result<()> {
    
    let mut unzipper = GzDecoder::new(File::open(&self.file_path)?);
    let mut uncompressed: Vec<u8> = Vec::new();
    unzipper.read_to_end(&mut uncompressed)?;

    let mut archive = Archive::new(uncompressed.as_slice());
    let mut watch_schema: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();

    // first, let's get REV and watch files
    for file in archive.entries()? {
      let mut file = file?;

      let filename = file.path()?.into_owned().to_str().unwrap().to_string();
      
      if filename.contains("REV") || filename.contains("watch") {

        let mut buf = String::new();
        file.read_to_string(&mut buf)?;

        if filename.contains("watch") {
          watch_schema = serde_json::from_str(&buf).unwrap();
        } 

        doc.insert(filename, buf);

      } 
    }
    

    // need get files that REV and watch need
    let watch_needed_files: Vec<&String> = watch_schema.as_object().unwrap().keys().collect();
    let mut rev_needed_files: HashMap<String, String> = HashMap::new();
    utils::read_file_into_hash(doc.get("REV").unwrap().as_str().unwrap(), None, &mut rev_needed_files)?;
    let rev_needed_files: Vec<&String> = rev_needed_files.keys().collect();

    let get_dump_files = watch_needed_files.contains(&&"dump".to_string());
    
    let mut archive = Archive::new(uncompressed.as_slice());
    for file in archive.entries()? {
      let mut file = file?;

      let filename = file.path()?.into_owned().to_str().unwrap().to_string();
      
      if watch_needed_files.contains(&&filename) || 
      rev_needed_files.contains(&&filename) || 
      (get_dump_files && filename.contains("dump")) {

        let mut buf = String::new();
        file.read_to_string(&mut buf)?;
        doc.insert(filename, buf);   

      } 
    }

    // Can go through watch schema to get out all desired values
    // outer loop goes through different files on watch list
    // middle loops goes through the lines of that file
    // inner loop checks each line to see if it contains one of the variables we are looking for.
    for f in watch_schema.as_object().unwrap().keys() {

      if f.contains("dump") { continue; } // skip dump files here bc these are not normal

      let file_contents = doc.get(f).unwrap();
      // watch_schema[f]["variables"].as_object().unwrap().keys().collect();
      let mut vars: Vec<&String> = Vec::new();
      let mut special_vars: Vec<&str> = Vec::new();
      for v in watch_schema[f]["variables"].as_object().unwrap().keys() {
        if watch_schema[f]["variables"][v]["type"] == "thermo_log" {
          special_vars.push(v);
        } else {
          vars.push(v);
        }
      }

      // getting the normal variables first
      for l in file_contents.as_str().unwrap().lines() {

        let line: Vec<&str> = l.split_whitespace().collect();

        for var_name in &vars {

          match line.iter().position(|&x| x == var_name.as_str()) {
            Some(v) => {
              let val_pos = v + 1;
              let var_type = watch_schema[f]["variables"][var_name]["type"].as_str().unwrap();

              match var_type {

                "int" => {
                  // this will skip occurrences of the watch variable with the wrong type. Kind of a hack for now
                  let val = line[val_pos].parse::<i64>();
                  match val {
                    Ok(v) => { watch_values.insert(var_name.to_string(), v); },
                    Err(_e) => println!("Incorrect type, skipping")
                  };
                },
                "float" => {
                  let val: f64  = line[val_pos].parse().unwrap();
                  watch_values.insert(var_name.to_string(), val);
                },
                "long_string" => { // string with spaces
                  watch_values.insert(var_name.to_string(), line[val_pos..].join(" "));
                },
                "string" => {
                  watch_values.insert(var_name.to_string(), line[val_pos]);
                },
                "keywords" => {
                  let mut keyword_doc = Array::new();
                  for word in line[val_pos..].iter() {
                    keyword_doc.push(Bson::String(word.to_string()));
                  }
                  watch_values.insert("keywords", keyword_doc);
                },
                &_ => { // anything is simple string
                  println!("Error! Invalid watch type {}", var_type);
                }
      
              };
            },
            None => (),
          };
        }
      }

      // now getting special variables
      for svar in special_vars {

        match watch_schema[f]["variables"][svar]["type"].as_str().unwrap() {
          "thermo_log" => {
            let mut thermo_keys: Vec<&str> = Vec::new(); // this vec is used to keep the correct order of data
            let mut thermo_data = Document::new(); //treat like a HashMap<&str, Vec<f64>>
            let mut read_mode = false;
            let mut read_count = 0;

            for l in file_contents.as_str().unwrap().lines() {

              let line: Vec<&str> = l.split_whitespace().collect();

              // need to know when to stop. So will break when we no longer find numbers
              // can clear out vec and doc to use for other runs
              if read_mode {
                match line[0].parse::<f64>() {
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
                for s in line {
                  thermo_keys.push(s);
                  let  v: Vec<f64> = Vec::new();
                  thermo_data.insert(s, v);
                }
                read_mode = true;
                continue;
              }

              if read_mode {
                for (i,s) in line.iter().enumerate() {
                  thermo_data.get_array_mut(thermo_keys[i]).unwrap().push(Bson::Double(s.parse().unwrap())); // yea bson is wild
                }
              }

            }
          },

          &_ => panic!("Unknown special variable")

        };

        
      }


      // just remove files not marked for uploading
      if watch_schema[f]["upload"].as_i64().unwrap() == 0 {
        doc.remove(f);
      }

    }

    // this marks dump files for removal from the doc
    let mut dump_removal: Vec<String> = Vec::new();
    
    // parse dump files
    if get_dump_files  && watch_schema["dump"]["parse"] != 0 {
      for f in doc.keys() {
        if f.contains("dump") {

          let file_contents = doc.get(f).unwrap();
          dump_removal.push(f.to_string());
          let mut dump_data = Document::new();
          let mut lines = file_contents.as_str().unwrap().lines().peekable();
          let mut timestep: &str = "0";

          loop {

            let line = match lines.next() {
              Some(l) => l,
              None => break
            };

            // stopping main loop only on ITEMs
            if !line.contains("ITEM:"){ continue; }
            let line: Vec<&str> = line.split("ITEM:").collect();

            // i'm assuming that the ITEM name is all caps
            // everything else is extra parameters
            let mut item = String::new();
            for c in line[1].chars() {
              if c.is_lowercase() {
                break;
              } else {
                item.push(c);
              }
            }
            let item = item.trim();

            let params: Vec<&str> = line[1].split(&item).collect();
            let params: Vec<&str> = params[1].split_whitespace().collect();

            if item.contains("TIMESTEP") {
              
              timestep = lines.next().unwrap();
              dump_data.insert(timestep, Document::new());
              continue;

            } else if params.len() > 0 { // assuming this will be a new subdocument

              dump_data.get_document_mut(timestep).unwrap().insert(item, Document::new());

            } else { // no params assuming that this something with a single line of info like "NUMBER OF ATOMS"

              dump_data.get_document_mut(timestep).unwrap().insert(item, lines.next().unwrap().parse::<f64>().unwrap());
              continue;

            }

            let mut sub_doc_count = 0;
            // if you are here then you have potentially several more lines of data
            loop {

              // peek ahead to see if we have an ITEM line
              match lines.peek() {
                Some(l) => {
                  if l.contains("ITEM:") { break; }
                },
                None => {
                  break;
                }
              };

              let l: Vec<&str> = lines.next().unwrap().split_whitespace().collect();
              let mut sub_doc = Document::new();
              for (i,line_item) in l.iter().enumerate() {
                sub_doc.insert(params[i].to_string(), line_item.parse::<f64>().unwrap());
              }
              dump_data.get_document_mut(timestep).unwrap().get_document_mut(&item).unwrap().insert(sub_doc_count.to_string(), sub_doc);
              sub_doc_count +=1 ;
            }

          }
          watch_values.insert(f, dump_data);
        }
      }

      for f in dump_removal {
        doc.remove(f);
      }

    }
    

    Ok(())
  }

  async fn get_file_diffs(&self, diffs:&mut Document, file_doc: &Document, rev_file_hash: HashMap<String, String>, db_name: &str) -> std::result::Result<(), mongodb::error::Error> {
    // first getting entry whose id matches the new parent id
    let parent_id = rev_file_hash.get("parent_id").unwrap();  
    let coll = parent_id.split(':').next().unwrap();
    
    if parent_id != "*" {

      // get parent REV hash
      let mut res = Connection::simple_db_query(&self.db_client, "id", parent_id, db_name, coll, Some(doc! {"files": 1})).await;

      // checks for case where parent id no longer exists
      let parent = match res.try_next().await? {
        Some(v) => v,
        None => {
          println!("No matching parent id in database, may have incorrectly deleted a record");
          return Ok(())
        }
      };

      let parent_rev = parent.get("files").unwrap().as_document().unwrap().get("REV").unwrap().as_str().unwrap();
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
    let coll_name = &self.conn.collection;

    // inserting general upload metadata
    parent_doc.insert("upload_name", &self.conn.filename);
    parent_doc.insert("upload_path", &self.file_path);
    parent_doc.insert("upload_time", chrono::offset::Utc::now());
    
    // Decompressing file and getting tracked and REV files
    let mut file_doc = Document::new();
    let mut watch_values = Document::new();

    self.decompress_data(&mut file_doc, &mut watch_values).expect("Decompression failed");
    let mut rev_file_hash = HashMap::new();
    utils::read_file_into_hash(file_doc.get("REV").unwrap().as_str().unwrap(), None, &mut rev_file_hash)?;
    parent_doc.insert("id", rev_file_hash.get("id").unwrap());
    parent_doc.insert("parent_id", rev_file_hash.get("parent_id").unwrap());

    // Calculating diffed files
    let mut diffs = Document::new();
    self.get_file_diffs(&mut diffs, &file_doc, rev_file_hash, db_name).await?;

    parent_doc.insert("watch", watch_values);
    parent_doc.insert("files", file_doc);
    parent_doc.insert("diffs", diffs);
    
    
    let db = self.db_client.database(db_name);
    let collection = db.collection::<Document>(coll_name);
    collection.insert_one(parent_doc, None).await?;

    Ok(())
    
  }

}