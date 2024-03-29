# Rust_Logger

Please email any questions or comments directly to tegrubbs1@gmail.com

## A git-like utility for easily tracking file changes

My past experiences with computational experiments - in AI, physics, engineering, etc - have shown that it is very easy to lose track of what things have changed in your configs, code, or input data. At the start of a new moloecular dynamics project in mid-2022 I have decided that it is worth investing some time into creating a system which can effortlessly log any changes that might occur in a project.

You might ask "well why not use git?". You certainly could use git to track these types of changes but git is more meant for more significant code-base changes that are more or less permanent. In computational science you might want to only change 1 or 2 variables and see how that changes the result. This can quickly generate thousands of git commits that would be impossible to keep track of or analyze using vanilla git or github. As computational scientists we need to easily visualize the relationships between potentially thousands or millions of experimental inputs and outputs. This is what has motivated the current project.

But this is not limited to simulation projects, it can also be used to log anything. For example, I am currently using to keep track of various notes that I write.

## Quick Install
Downloads for the relevant executables, README, and installation scripts can be found on my personal website at:

https://taylorgrubbs.online/downloads/

You only actually need to download and run the relevant installers to use the software. There are no software prerequisites needed to run the installers, but you will need superuser privileges:

```bash
# Server installation
chmod +x log_server_installer.sh 
./log_server_installer.sh

# log installation
chmod +x log_installer.sh 
./log_installer.sh
```

The installer also functions as an update script. If you run it again, the latest version of the code is downloaded and reinstalled on your system.

You can also uninstall the software with the `-u` option:
```bash
./log_installer.sh -u # This will remove the software from the system
```

## Contents

- [Rust_Logger](#rust_logger)
  - [A git-like utility for easily tracking file changes](#a-git-like-utility-for-easily-tracking-file-changes)
  - [Quick Install](#quick-install)
  - [Contents](#contents)
  - [Quick Help](#quick-help)
    - [`Commands`:](#commands)
    - [`watch` file schema:](#watch-file-schema)
  - [How it works](#how-it-works)
- [Setup - `log_server`](#setup---log_server)
  - [Setup - MongoDB](#setup---mongodb)
  - [Setup - log_server: Prerequisite TLS](#setup---log_server-prerequisite-tls)
  - [Setup - log_server](#setup---log_server-1)
  - [Setup - log_server config](#setup---log_server-config)
- [Setup - log](#setup---log)
  - [Setup - log](#setup---log-1)
  - [Setup - log config](#setup---log-config)
- [`log`](#log)
  - [`log` - the command-line utility](#log---the-command-line-utility)
  - [`log` - registration](#log---registration)
    - [Usage:](#usage)
  - [`log` - upload](#log---upload)
    - [Usage:](#usage-1)
  - [Example](#example)
- [`log` options](#log-options)
  - [`log` - `coll` and `name`](#log---coll-and-name)
  - [`log` - `update`](#log---update)
  - [`log` - `watch` file](#log---watch-file)
  - [`log` - `dump` files](#log---dump-files)
  - [log - clean](#log---clean)
- [How to actually use this data](#how-to-actually-use-this-data)
- [`log_server` - Web interface](#log_server---web-interface)
  - [Pandoc and Markdown](#pandoc-and-markdown)
- [Modifying the code yourself](#modifying-the-code-yourself)

## Quick Help
### `Commands`:
>*User input is denoted by angle brackets <>*

- `log <lammps command>` - will execute `<lammps command>` and upload results to server. Command must include "-in" followed by a lammps input file. 
  - Example: `log mpirun -np 4 lmp -in in.crack`
- `log -c < file / directory / . >` - Will compress and upload current directory to server. If given a file, will compress the directory containing said file.
  - Example: `log -c lammps/examples/crack/`
- `log options`:
  - `--coll <collection name>` - specifies collection where file will go
  - `--name <upload name>` - specific name of file or `upload_name`
  - `--force` - Will force upload whenever `parent_id` is not in database. May cause a break in revision chain
  - `--update` - Will pull the latest upload from the collection and unpack it into your chosen directory
- `log clean` - Will remove any "dead" files deleted from database but still on the server filesystem.
  - Example: `log clean`

### `watch` file schema:

```json
{
  "<file name>": 
  {

    "upload": "bool",

    "variables": 
    {
      "<variable name: string>":
      {
        "type": "<type>"
      }
    }
  },

  "dump": 
  {
    "parse": "bool"
  }

}
```
Valid types are:

- string
- long_string
- float
- int
- thermo_log
- keywords

And yes I know that JSON doesn't have boolean types, for `"bool"` I actually mean `1/0` for `true/false`, respectively.

## How it works 
The current system is broken into 2 parts - the `log` and `log_server`. `log` is a utility for users or automated programs to upload results to the `log_server`. The `log_server` is a webserver+database combo that receives data from the `log` and inserts it into a local MongoDB database. The communication between the client and server is encrypted via TLS to maintain confidentiality.

# Setup - `log_server`

## Setup - MongoDB
Rust_Logger depends on a MongoDB database for it's backend. Currently this database must be installed and configured manually by you, the user. However, this does not require much work. We only need to set up a Mongo server with basic authentication settings. 

I have compiled some notes on what is required to set up the database properly in `log_server/Extra_Setup_Help/MongoDb setup tricks.md`. Please check that out if you are stuck. Note that TLS setup in in the database settings is not required by Rust_Logger. That is only necessary if you intend on querying the MongoDB directly and want your connection to be secure.

And be sure to choose a good password for your admin user. This is needed later on.

## Setup - log_server: Prerequisite TLS
I will now describe the setup process of the `log_server`. `log_server` communicates with a `log` through http communication secured with TLS. This requires that the system running `log_server` contain an unexpired, valid TLS certificate. 

Now if you are running `log_server` on a local network you will need to create a self-signed certificate and manually add the certificate to all client machines' trusted lists. I describe how to do this in `log_server/Creating self-signed certificates.txt` but you can also find plenty of manuals on how to do this online. 

If however you are running the server on a internet-accessible server, then you likely already have a valid TLS certificate provided by an organization like Let's Encrypt (https://letsencrypt.org/). You can then use this certificate and not have to worry about modifying your clients' trust lists. 

## Setup - log_server
Installing the `log_server` is quite trivial. Just run the install script at `log_server/install.sh`:

![Alt text](imgs/server_install.png)

This will install the executable `tls_server` to `/usr/bin/` and attempt to start the service as `log_server.service`. It will of course fail because it does not know where your TLS certificates are located.

## Setup - log_server config
As the printed install messages indicate, you must first set up the server config. This will be located in a new hidden folder in your home directory called `.log_server/config`. It looks like this:

```
server_port : 1241
cert_path : /home/tedwing/.log_server/myserver.crt
key_path : /home/tedwing/.log_server/myserver.key
data_path : /home/tedwing/.log_server/data/
database : LAMMPS
css : https://a-template-url
```

Not much going on there. The options are pretty self-explanatory but I will explain them here:

- `server_port` - The port where your server will listen for http requests from clients. Could be anything you want but probably want it above 1023 to avoid colliding with the [Well-known ports](https://en.wikipedia.org/wiki/List_of_TCP_and_UDP_port_numbers#Well-known_ports).
- `cert_path` - Location of TLS certificate. Point to wherever your self-signed crt file is located - or if have a certificate from Lets Encrypt yours will be at `/etc/letsencrypt/live/<your domain name>/fullchain.pem`.
- `key_path` - Location of TLS private key. Point to wherever your self-signed key file is located - or if have a certificate from Lets Encrypt yours will be at `/etc/letsencrypt/live/<your domain name>/privkey.pem`.
- `data_path` - Location where uploaded files will be stored on disk. The default location should be fine.
- `database` - Name of the MongoDB database that the server will create for you. This also can be anything you want. 
- `css` - (Optional) The URL of a css stylesheet. This will be used to render certain web pages in the [Web interface](#log_server---web-interface).

After this is properly set up we can start the service with systemctl:

![Alt text](imgs/service_startup.png)

# Setup - log

## Setup - log
With a server now running we can now move to setting up a `log`. This is considerably simpler since there are no prequisites or services running. Again here there is also a `install.sh` script- but this time in the `log/` folder:

![Alt text](imgs/client_install.png)

Considerably less flashy than the server install. 

## Setup - log config
This also installs a config to a hidden folder in the home folder at `~/.log/config`:

```
Username : tayg
Server : localhost:1241
tracked_files : in.
```

Even more boring than the server config!

- `Username` - The username that will register this machine with the server. Set it to whatever you want (Might change this in the future to just use the system name so that you don't have to create a username for each machine manually)
- `Server` - This is the site + port of the machine  where the `log_server` is running. So if the server was running at example.com on port 1241 I would put `example.com:1241` here.
- `tracked_files` - This denotes a list of filetypes that `log` should monitor for changes. This can be a file extension, file prefix, or just some common substring found in your files. Different types are separated by commas so to track multiple files this would look like "`tracked_files : .log, .txt, .csv`"

This covers the basic setup required for Rust_Logger to operate. We can now do *fun things*.

# `log`

## `log` - the command-line utility
When you run the `log` install script, you are actually installing a tool called `log` which can run from the linux terminal. `log` is what you use for all interactions with Rust_Logger. I will now explain each of its core functions.

## `log` - registration
>*The following is not essential for the actual usage of Rust_Logger but may be helpful for your overall understanding of the program. But feel free to skip if you are impatient.*

### Usage: 

`log <anything>`

When you first run `log` on a system, you cannot actually access the database to insert or read documents bc the system does not have a valid account. The first thing that `log` does is to ask you for the admin password of the Mongo database. This would have been setup in the [Setup - MongoDB](#setup---mongodb) phase. The system will then use this password to authenticate to the server, which will then create a new user with full access to the `database` given in the server config. The username is the same `username` from the client config. 

The server returns an API key which is then stored on you client machine as `/etc/.Rust_Logger_Credentials`. Only the root user on your client system can access this file. `log` is an SUID binary which temporarily obtains root privileges to read from this file. All other operations occur with your usual user permissions.

This username + API key is what is used to further authenticate the client system to the server. 

If everything runs properly this whole process is nearly instantaneous and unnoticeable to the end-user.

If for whatever reason your account is dropped from the database, you will need to re-register. To do this you will first need to delete the `.Rust_Logger_Credentials` file. Then the next run of `log` will perform the registration process again.

Connections to different servers are also supported. Whenever you use a new `Server` in the `log` config, the registration will run again as explained above.

## `log` - upload
>*You absolutely MUST read this to be able to use the Rust_Logger in any meaningful way.*

### Usage:

`log -c <file or directory>`

or

`log <lammps command>`

We now get to the real meat of the Rust_Logger. I think it is best to show how it works through an example.

## Example

Let's start with a simple example from the LAMMPS molecular dynamics repository. Because of it's funny name I have been testing all of this with the `crack` simulation (it simulates a crack forming between atoms). Here we see the directory containing the input file:

![Alt text](imgs/crack_folder.png)

Pretty boring. Just a simple input script. I can run the simulation easily with 

`lmp -in in.crack`

Doing that will generate an output file - log.lammps.

![Alt text](imgs/crack_after_run.png)

We would like to log this information with `log`. We can easily do this with:

`log -c .`

This will log the entire directory. But it has also created a new file in the directory:

![Alt text](imgs/after_log.png)

The REV file (short for "revision") is how the logger keeps tracks of changes made in the directory. It does this by recording hashes of the files in the directory. When you run `log` again it recalculates these hashes to see if they are different from what is in the REV file. If so, then `log` knows that there has been a change and that this needs to be recorded.

Let's look at what the previous command uploaded to Mongo:

![Alt text](imgs/mongodb_upload.png)

From `upload_name` it looks like we have uploaded a compressed tar.gz version of the directory- which is exactly what has occurred. The `log` simply compresses the entire directory and sends it to `log_server`. 

Below that we also see `id` and `parent_id`. These fields are used to track changes between simulations and track the progress of these changes. `id` is simply a combined hash of all files in the directory, with the directory name prepended (that's also the MongoDB collection name). `parent_id` is the id of the previous version of this simulation. Since this was the first upload to this collection, the `parent_id` is `*` which lets us know that this is the root entry.

If you try to log the directory again we are given an error from `log`:

![Alt text](imgs/same_file_error.png)

The `log_server` uses the `id` to stop us from sending the exact set of files twice. Since we have made no changes, this hash has not changed.

Let's see what happens when add a new arbitrary file:

![Alt text](imgs/arb_file.png)

But this will also fail. Why is this? Remember that `tracked_files` config setting? You can check it here [Setup - log config](#setup---log-config). By default Rust_logger will only monitor `in.` files which are the common prefix for LAMMPS input files. Only changes to these files will cause `log` to take notice. It's kind of like an inverse of the `.gitignore` file. `log` ignores everything __NOT__ in the `tracked_files` list.

Let's change the input file by changing the timestep from 0.003 to 0.001 and then reupload:

![Alt text](imgs/change_to_input_file.png)

The second entry shows a new `id` and the `parent_id` matches the previous run's `id`.

Rust_Logger also records the specific changes that are made. This is recorded in the `diffs` object:

![Alt text](imgs/diff.png)

We can also access the entire input file directly in the `files` object:

![Alt text](imgs/files_obj.png)

`log` will prevent a drop in the revision chain by erroring if it cannot find the parent of your current directory. If I were to delete everything in our new `crack` collection on MongoDB and then try to reupload the directory I would get the following message:

```
thread 'main' panicked at 'Error: Previous record not found in database, revert changes or delete REV file to create a new branch', src/main.rs:44:5
```

This error can only occur if something was deleted in the MongoDB. To fix this you will need to delete the current REV file so that `log` can start a new branch.

You can ignore this error however by providing the `--force` option in your command. This will guarantee that your files are uploaded.

# `log` options

## `log` - `coll` and `name`
When you upload a new directory for the first time you must specify the `--coll` option followed by a collection name. This will tell `log` where to upload this file. Subsequent uploads will reuse this collection, unless a new collection is specified.

You may also specify a specifc name for the upload with `--name`. This changes the `upload_path` and `upload_name` fields in the database. Any uploads without `--name` specified will default to the directory name. If a duplicate `upload_name` occurs, the server will append the current datetime to this name.

## `log` - `update`
This program feels a lot like git, so there must be the equivalent of a git pull command. There is. It can be executed by providing the `--update` option. When this option is present, `log` will go to the specified collection and download the latest upload from the server. These files will then be unpacked into your selected directory. 

However, if there are local changes in your local directory, this operation will fail - just like with git. Your changes will either need to be uploaded or removed before attempting the update again.



## `log` - `watch` file
Within the `files` object we can also see the REV file here. But notice that the other files - `log.lammps` and `test_file` - are not present. This is because these file types are not on the `tracked_files` list. However, there is a way for us to tell `Rust_Logger` to monitor them.

The purpose of Rust_Logger is not to just monitor file changes, but to also record specific simulation outputs. We can tell Rust_Logger to monitor certain outputs through the use of the `watch` file. 

The `watch` is actually a kind of schema that we define using JSON. We use it to define what additional files we want to upload to the `log_server` and specific variables that we might want to track from our simulation outputs. It is added to the directory by the user:

![Alt text](imgs/watch_file_in_dir.png)

For our current `crack` example, a `watch` file might look like this:

```json
{
  "log.lammps": 
  {

    "upload": 1,

    "variables": 
    {
      "atom_style":
      {
        "type": "string"
      },

      "timestep":
      {
        "type": "float"
      },

      "dimension":
      {
        "type": "int"
      },

      "pair_style":
      {
        "type": "long_string"
      },

      "thermo_data": 
      {
        "type": "thermo_log"
      },

      "keywords":
      {
        "type": "keywords"
      }
    }

  }
}
```
>*If your eyes have glazed over, that's ok. The author is thinking of how to make this more user-friendly*

The `watch` file is made up of different objects where the outermost key is the filename. This tells `log_server` to pay attention to this particular file. In this case we only reference one file: `log.lammps`.

Within the file object we have 2 parameters `"upload"` and`"variables"`. `"upload"` is a boolean which tells `log_server` whether or not to include the entire file in the `files` object within the database. 

`"variables"` is a list that tells `log_server` fields to extract from the file. It does this by looking for the appearance of the variable string within the file, and then extracting the value that appears after the string on the same line. We tell `log` the variable type with the `"type"` key.

The variables given here may not be useful in any output analysis but they give examples of every possible type currently supported in Rust_Logger. Here is the corresponding entry within mongo:

![Alt text](imgs/watch_data.png)

`"thermo_log"` is a special type which refers to the thermodynamic data printed by LAMMPS:

![Alt text](imgs/thermo_data.png)

In the database it looks like this:

![Alt text](imgs/thermo_data_db.png)
![Alt text](imgs/thermo_data_db_expanded.png)

`keywords` is another special type that lets you add tags to your files. In the above example the `log_server` would look for the appearance of they "keywords" string and then add each word after that as a separate entry in the database. 

For example, in my log.lammps file I might have 

```
keywords crack sim test
```

In the database this will show up as

![Alt text](imgs/keywords_example.png)

This will hopefully be useful in a future search functionality.

>*You might be wondering, "well, what if I have multiple occurences of the same string in my output files? How will Rust_Logger handle those?"  
> The answer is ... it doesn't. It will just use the last appearance of that variable for the value it logs.  
> However the thermo_log type can accurately handle this situation by making as many thermo_log entries as needed.*

## `log` - `dump` files
It is possible to log dump files with Rust_Logger as well. In the `watch` file you can add `"dump"` as a file.

```json
{
  "log.lammps": 
  {
    ...
  },

  "dump": 
  {
    "parse": 1
  }
}
```

The `"parse"` option tells the logger to parse the dump file into a large dump object. Otherwise, the entire dump file will be just  uploaded as a single string. And example in Mongo is shown below:

![Alt text](imgs/dump_example.png)

Compared to the original file:

![Alt text](imgs/dumpfile.png)

>*Note this is very much a work in progress and every possible LAMMPS parameter may not be parsed correctly. For example, it is known that BOX BOUNDS is definitely not handled correctly.*

>*It is also not recommended to try and view dump file outputs through MongoDB Compass. The large size of the dump files makes the interface very laggy. However, querying the dump file data through other means is quite fast.*

## log - clean
One final aspect of the Rust_Logger is the file storage. When you log a directory with `log` it sends a compressed version of the directory to the server. These files reside in the `data` folder specified in the `data_path` config:

![Alt text](imgs/data_path_ex.png)

This allows you to easily retrieve any of the files if desired. 

However sometimes you may want to delete something from the MongoDB. This will not trigger a removal of the actual file from the server filesystem. To clear out these "dead" files, you can run the `log clean` command. This will prompt you for the Mongo admin password and then go find all the files within `data_path` which no longer have a matching entry in the database. These files are then deleted. 

# How to actually use this data
I've talked a lot about how to upload your data to a database using the Rust_Logger, but I haven't really shown you what you can do with this data afterwards. I might add some examples of this later on but right now it is up to you.

I would recommend using [Python](https://www.mongodb.com/languages/python) to query and use your data, but you can use whatever language or tool you want.

# `log_server` - Web interface
>*The  Web interface is an entirely optional application built on top the `log_server`. You can use Rust_logger without ever accessing the web page.*

There is simple a html-based interface which let's you download files from the server. Simply navigate to the url of the server (`Server` in the client config). You will be greeted by a simple web page that prompts you for the database admin password. You can then navigate between your collections and click to download anything that you have uploaded

![Alt text](imgs/web_interface_example.png)

The site does use cookies to make sure you are properly authenticated and they currently expire after 10 minutes. After that you will have to log in again.

The appearance of the site may be enhanced by providing a css stylesheet url in the `log_server` config's `css` option.

## Pandoc and Markdown

You can also view the contents of your uploads. If [pandoc](https://pandoc.org/) is installed on your server, the `log_server` will attempt to convert any markdown (.md) files to html. The server will attempt to use a template called `"pandoc_template.html5"` insde your pandoc templates folder. You will need to create this file for yourself if you want to render markdown files a certain way.

The server also loads the [MathJax](https://www.mathjax.org/) library to render any math embedded in the document. A valid internet connection on the client is required for this to work since it retrieves the js file from the jsdelivr cdn. This was done mostly for me to view my own notes through the server.  

# Modifying the code yourself
Building the code simply requires a modern Rust installation as far as I know. To effectively test the server code locally you will need to create a self-signed TLS certificate. I give instructions on this in `log_server/Extra_Setup_Help/Creating self-signed certificates.md`. But there are plenty of other help-articles on this.

The `root_suid.sh` scripts should be used to build the code. The scripts simply run `cargo build` and then convert the executables into suid binaries. Just running `cargo build` will not let you run any of the code since both perform operations requiring root privileges.

Once the server is running locally, the `Server` option in your client config should be something like `localhost:<server_port>`. A more concrete example of local configs for development would look like this:

log_server config
```
server_port : 1241
cert_path : /home/tedwing/.log_server/myserver.crt
key_path : /home/tedwing/.log_server/myserver.key
data_path : /home/tedwing/.log_server/data/
database : LAMMPS
css : 
```

log config
```
Username : tayg
Server : localhost:1241
tracked_files : in., .md, .bib, .json, .ipynb, .pdf, .odt
```

Release builds can be made by running the `make.sh` script followed by the executable name:
```bash
./make.sh log
./make.sh log_server
```
This will create new builds in the `log/build/` or `log_server/build/` directories.
