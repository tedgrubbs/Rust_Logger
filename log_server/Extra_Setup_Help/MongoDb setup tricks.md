# MongoDB Setup Instructions


>*Note that MongoDB currently only supports Ubuntu 20, 18, 16 as described in the below install link. Some dumb workarounds are required for it to work with Ubuntu 22*

# Installation 
Should be able to easily follow the MongoDB docs for installation: https://www.mongodb.com/docs/manual/tutorial/install-mongodb-on-ubuntu/

After the general install finishes you will probably need to make the mongodb user owner of the following directory/file in order to start up
```
sudo chown -R mongodb:mongodb /var/lib/mongodb
sudo chown mongodb:mongodb /tmp/mongodb-27017.sock
```


# Security setup 

## [Security check list](https://www.mongodb.com/docs/manual/administration/security-checklist/)

## [Authentication setup](https://www.mongodb.com/docs/manual/tutorial/configure-scram-client-authentication/#std-label-create-user-admin)

### Creates a new admin with the ability to create users and change/delete any database
```
use admin
db.createUser(
  {
    user: "admin",
    pwd: passwordPrompt(), // or cleartext password
    roles: [
      { role: "userAdminAnyDatabase", db: "admin" },
      { role: "readWriteAnyDatabase", db: "admin" },
      { role: 'dbAdminAnyDatabase', db: 'admin' }
    ]
  }
)
```

## [TLS Setup](https://www.mongodb.com/docs/manual/tutorial/configure-ssl/)

```
# this is in /etc/mongod.conf
net:
   tls:
      mode: requireTLS
      certificateKeyFile: /etc/ssl/mongodb.pem
```

Can make a pem file by concatenating .crt and .key files.

For Let's Encrypt certs can do this:
```bash
cat fullchain.pem privkey.pem > mongo.pem
# These are inside /etc/letsencrypt/live/<domain>/
```

Will need to then change the owner of this new pem file to the mongodb user and make sure no one else can read it, and make sure mongodb can actually get to it by placing it in a directory where it has access.

>*Note that TLS connections through the driver seem to not work at all for self-signed certs. Will need to use properly verified cert if TLS in
desired in production*

>*When not authenticating to admin, need to specify the database that the user is attached to in the "authSource" field.*

>*When on a public server need to change the Bind_Ip to 0.0.0.0 or else it will only listen to connections from the localhost interface (this is also in /etc/mongod.conf)*
